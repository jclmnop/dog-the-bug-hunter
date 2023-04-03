use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use wasmcloud_interface_endpoint_enumerator::{Subdomain, Subdomains};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CrtShEntry {
    name_value: String,
}

pub async fn enumerate_subdomains(domain: &str) -> Result<Subdomains> {
    let url = format!("https://crt.sh/?q=%25.{domain}&output=json");
    let res = reqwest::get(&url).await?;

    let crt_sh_response: Vec<CrtShEntry> = res.json().await.map_err(|e| {
        anyhow::anyhow!("\nError decoding crt.sh response: {e}")
    })?;

    Ok(parse_crt_sh_response(crt_sh_response)
        .into_iter()
        .map(|subdomain| Subdomain {
            open_ports: vec![],
            subdomain,
        })
        .collect())
}

fn parse_crt_sh_response(crt_sh_response: Vec<CrtShEntry>) -> Vec<String> {
    let subdomains: HashSet<String> = crt_sh_response
        .into_iter()
        .flat_map(|entry| {
            entry
                .name_value
                .split('\n')
                .map(|subdomain| subdomain.trim().to_string())
                .collect::<Vec<String>>()
        })
        .filter(|subdomain| !subdomain.contains('*'))
        .collect();

    subdomains.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // fn start_logger() {
    //     let _ = env_logger::builder().try_init();
    // }

    #[test]
    fn test_parse_crt_sh_response() {
        let raw_response = r#"
        [
          {
            "issuer_ca_id": 1397,
            "issuer_name": "C=US, O=DigiCert Inc, OU=www.digicert.com, CN=DigiCert SHA2 High Assurance Server CA",
            "common_name": "*.github.com",
            "name_value": "*.github.com\ngithub.com",
            "id": 7264603,
            "entry_timestamp": "2015-04-22T16:20:13.18",
            "not_before": "2015-03-23T00:00:00",
            "not_after": "2017-03-27T12:00:00",
            "serial_number": "086cb1fa40c81e2512eacf2ea9444e53"
          },
          {
            "issuer_ca_id": 1397,
            "issuer_name": "C=US, O=DigiCert Inc, OU=www.digicert.com, CN=DigiCert SHA2 High Assurance Server CA",
            "common_name": "www.github.com",
            "name_value": "*.github.com\ngithub.com\nwww.github.com",
            "id": 12168459,
            "entry_timestamp": "2016-01-22T12:10:42.119",
            "not_before": "2016-01-20T00:00:00",
            "not_after": "2017-04-06T12:00:00",
            "serial_number": "077a5dc3362301f989fe54f7f86f3e64"
          },
          {
            "issuer_ca_id": 1397,
            "issuer_name": "C=US, O=DigiCert Inc, OU=www.digicert.com, CN=DigiCert SHA2 High Assurance Server CA",
            "common_name": "support.enterprise.github.com",
            "name_value": "support.enterprise.github.com",
            "id": 12061525,
            "entry_timestamp": "2016-01-17T11:20:30.986",
            "not_before": "2016-01-14T00:00:00",
            "not_after": "2018-01-18T12:00:00",
            "serial_number": "067d421fee3ee376cfcae377f6864731"
          }
        ]
        "#;

        let crt_sh_response: Vec<CrtShEntry> =
            serde_json::from_str(raw_response).unwrap();

        let subdomains = parse_crt_sh_response(crt_sh_response);

        assert_eq!(subdomains.len(), 3);
        assert!(subdomains.contains(&"github.com".to_string()));
        assert!(subdomains.contains(&"www.github.com".to_string()));
        assert!(
            subdomains.contains(&"support.enterprise.github.com".to_string())
        );
    }

    // #[tokio::test]
    // async fn handles_empty_response() {}

    // #[tokio::test]
    // async fn test_enumerate_subdomains() {
    //     // start_logger();
    //     let subdomains = enumerate_subdomains("github.com").await.unwrap();
    //     info!("{}", serde_json::to_string_pretty(&subdomains).unwrap());
    //     // assert_eq!(subdomains.len(), 3);
    //     assert!(subdomains.contains(&Subdomain {
    //         open_ports: vec![],
    //         subdomain: "github.com".to_string()
    //     }));
    //     assert!(subdomains.contains(&Subdomain {
    //         open_ports: vec![],
    //         subdomain: "www.github.com".to_string()
    //     }));
    //     assert!(subdomains.contains(&Subdomain {
    //         open_ports: vec![],
    //         subdomain: "support.enterprise.github.com".to_string()
    //     }));
    // }
}
