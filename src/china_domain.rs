use crate::GLOBAL_DATA;
use std::collections::HashMap;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{Read, Write};

pub struct DomainCache {
    list: HashMap<String, bool>,
    cache_path: String,
}

impl DomainCache {
    pub fn init(cache_path: Option<String>) -> Result<DomainCache, Box<dyn Error>> {
        let cache_path = cache_path.unwrap_or("domain_cache.txt".to_string());
        let mut cache_file = OpenOptions::new()
            .write(true)
            .create(true)
            .read(true)
            .open(&cache_path)?;

        let mut cache_content = String::new();
        cache_file.read_to_string(&mut cache_content)?;

        let list = if cache_content.len() == 0 {
            HashMap::new()
        } else {
            let hash_set: HashMap<String, bool> = serde_json::from_str(&cache_content).unwrap();
            hash_set
        };

        Ok(DomainCache { list, cache_path })
    }

    pub(crate) fn sync_to_file(&self) -> Result<(), std::io::Error> {
        let mut cache_file = OpenOptions::new().write(true).open(&self.cache_path)?;
        let result = cache_file.write_all(serde_json::to_string(&self.list)?.as_bytes());
        info!("China Domain Cache Saved");
        result
    }

    fn add_domain(&mut self, item: String, value: bool) -> Option<bool> {
        let result = self.list.insert(item, value);
        println!("Current List: {:?}", self.list);
        result
    }

    fn find_domain(&self, item: &String) -> Option<&bool> {
        self.list.get(item)
    }
}

pub fn is_china_domain(domain: &String) -> Result<bool, Box<dyn std::error::Error>> {
    let v: Vec<&str> = domain.split('.').collect();

    let suffix = v.get(v.len() - 2).ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Not a valid domain",
    ))?;

    match &suffix[..] {
        "cn" => Ok(true),
        "com" | "net" => is_beian_domain(domain),
        _ => Ok(false),
    }
}

fn is_beian_domain(domain: &String) -> Result<bool, Box<dyn std::error::Error>> {
    let v: Vec<&str> = domain.split('.').collect();

    let suffix = v.get(v.len() - 2).ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Not a valid domain",
    ))?;

    let name = v.get(v.len() - 3).ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Not a valid domain",
    ))?;

    let query_domain = format!("{}.{}", name, suffix);

    let mut cache = GLOBAL_DATA.lock().unwrap();

    let cache_result = cache.find_domain(domain);

    let result = match cache_result {
        Some(domain_cache_result) => *domain_cache_result,
        None => {
            let request_url = format!(
                "https://apidata.chinaz.com/CallAPI/Domain?key={}&domainName={}",
                std::env::var("CHINAZ_API_KEY")?,
                query_domain
            );

            let body = reqwest::blocking::get(request_url)?.text()?;
            let json_body: serde_json::Value = serde_json::from_str(&body)?;

            let api_state_code = json_body.get("StateCode").ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Not a valid domain",
            ))?;

            let result = api_state_code == 1;
            cache.add_domain(domain.clone(), result);

            result
        }
    };

    Ok(result)
}
