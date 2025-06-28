use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::json;

#[derive(Debug, Default, Clone)]
pub struct DiscoveredPools {
    pub raydium_pool_list: Vec<String>,
    pub raydium_cp_pool_list: Vec<String>,
    pub raydium_clmm_pool_list: Vec<String>,
    pub meteora_dlmm_pool_list: Vec<String>,
    pub meteora_damm_pool_list: Vec<String>,
    pub meteora_damm_v2_pool_list: Vec<String>,
    pub pump_pool_list: Vec<String>,
    pub whirlpool_pool_list: Vec<String>,
    pub solfi_pool_list: Vec<String>,
    pub vertigo_pool_list: Vec<String>,
}

const BITQUERY_ENDPOINT: &str = "https://streaming.bitquery.io/eap";
const DISCOVERY_QUERY: &str = r#"
query ($token: String) {
  Solana {
    DEXPools(
      orderBy: {descendingByField: \"Pool_Quote_PostAmountInUSD_maximum\"}
      where: {Pool: {Market: {BaseCurrency: {MintAddress: {is: $token}}}}}
    ) {
      Pool {
        Market {
          QuoteCurrency {
            Symbol
            Name
            MintAddress
          }
          MarketAddress
        }
        Dex {
          ProtocolFamily
        }
        Base {
          PostAmount(maximum: Block_Slot)
          PostAmountInUSD(maximum: Block_Slot)
        }
        Quote {
          PostAmount(maximum: Block_Slot)
          PostAmountInUSD(maximum: Block_Slot)
        }
      }
    }
  }
}
"#;

const MIN_LIQUIDITY_USD: f64 = 1_000.0;

pub async fn discover_pools(token: &str, api_key: &str) -> Result<DiscoveredPools> {
    let client = Client::new();
    let body = json!({"query": DISCOVERY_QUERY, "variables": {"token": token}});
    let resp = client
        .post(BITQUERY_ENDPOINT)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow!("GraphQL request failed: {e}"))?;
    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| anyhow!("Invalid GraphQL response: {e}"))?;

    let mut result = DiscoveredPools::default();
    let pools = value["data"]["Solana"]["DEXPools"].as_array().ok_or_else(|| {
        anyhow!("Unexpected GraphQL response structure")
    })?;

    for pool in pools {
        let addr = pool["Pool"]["Market"]["MarketAddress"].as_str().unwrap_or("").to_string();
        let protocol = pool["Pool"]["Dex"]["ProtocolFamily"].as_str().unwrap_or("").to_lowercase();
        let base_usd = pool["Pool"]["Base"]["PostAmountInUSD_maximum"].as_f64().unwrap_or(0.0);
        let quote_usd = pool["Pool"]["Quote"]["PostAmountInUSD_maximum"].as_f64().unwrap_or(0.0);
        let liquidity = base_usd + quote_usd;
        if liquidity < MIN_LIQUIDITY_USD {
            continue;
        }

        if protocol.contains("pump") {
            if result.pump_pool_list.len() < 5 {
                result.pump_pool_list.push(addr);
            }
        } else if protocol.contains("raydium clmm") {
            if result.raydium_clmm_pool_list.len() < 5 {
                result.raydium_clmm_pool_list.push(addr);
            }
        } else if protocol.contains("raydium cp") {
            if result.raydium_cp_pool_list.len() < 5 {
                result.raydium_cp_pool_list.push(addr);
            }
        } else if protocol.contains("raydium") {
            if result.raydium_pool_list.len() < 5 {
                result.raydium_pool_list.push(addr);
            }
        } else if protocol.contains("damm v2") {
            if result.meteora_damm_v2_pool_list.len() < 5 {
                result.meteora_damm_v2_pool_list.push(addr);
            }
        } else if protocol.contains("damm") {
            if result.meteora_damm_pool_list.len() < 5 {
                result.meteora_damm_pool_list.push(addr);
            }
        } else if protocol.contains("dlmm") {
            if result.meteora_dlmm_pool_list.len() < 5 {
                result.meteora_dlmm_pool_list.push(addr);
            }
        } else if protocol.contains("whirlpool") || protocol.contains("orca") {
            if result.whirlpool_pool_list.len() < 5 {
                result.whirlpool_pool_list.push(addr);
            }
        } else if protocol.contains("solfi") {
            if result.solfi_pool_list.len() < 5 {
                result.solfi_pool_list.push(addr);
            }
        } else if protocol.contains("vertigo") {
            if result.vertigo_pool_list.len() < 5 {
                result.vertigo_pool_list.push(addr);
            }
        }
    }

    Ok(result)
}

