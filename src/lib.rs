use crate::provider::*;
use crate::repo::*;
use anyhow::{anyhow, Error};
use argh::FromArgs;
use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use rouille::Request;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::marker::Send;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

pub mod provider;
pub mod repo;

/// Top-level command.
#[derive(FromArgs, PartialEq, Debug)]
pub struct Command {
    /// strategy in selecting what telecom provider handles a verification attempt
    #[argh(option)]
    pub balancer: BalancerType,

    /// the port that the telecom verification service runs on
    #[argh(option, short = 'p', default = "String::from(\"5000\")")]
    pub port: String,
}

#[derive(PartialEq, Debug)]
pub enum BalancerType {
    RoundRobin,
    Best,
}

impl FromStr for BalancerType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rr" | "round-robin" => Ok(Self::RoundRobin),
            "b" | "best" => Ok(Self::Best),
            _ => Err(anyhow!("Invalid client_mode: {}", s)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct VerificationRequest {
    number: String,
    #[serde(with = "ts_milliseconds")]
    time: DateTime<Utc>,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct VerificationResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl VerificationResponse {
    pub fn to_string(&self) -> String {
        match serde_json::to_string(self) {
            Ok(s) => s,
            Err(_) => "verification response serialization error".to_string(),
        }
    }
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct RankResponse {
    rank: Vec<(String, f32)>,
}

pub struct VerificationServer {
    carriers: Vec<Box<dyn TelecomProvider>>,
    balancer: Box<dyn Balancer>,
    repo: Box<dyn VerificationRepo>,
}

impl VerificationServer {
    pub fn new(
        client_mode: BalancerType,
        carriers: Vec<Box<dyn TelecomProvider>>,
        repo: Box<dyn VerificationRepo>,
    ) -> VerificationServer {
        let balancer = match client_mode {
            BalancerType::RoundRobin => Box::new(RoundRobinBalancer::new()),
            BalancerType::Best => unimplemented!("BestBalancer is not supported yet"),
        };
        Self {
            carriers,
            balancer,
            repo,
        }
    }

    pub fn handle_request(
        &mut self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, Error> {
        let carrier = match self
            .carriers
            .get(self.balancer.next_idx(self.carriers.len()))
        {
            Some(c) => c,
            None => {
                return Ok(VerificationResponse {
                    token: None,
                    error: Some("no carriers found".to_string()),
                })
            }
        };
        println!("request handled by: {}", carrier.get_name());
        let entry = carrier.verify(&request.number);
        self.repo.store_attempt(entry.clone())?;
        match entry.step {
            VerificationStep::Unreachable => Ok(VerificationResponse {
                token: None,
                error: Some("verification unsuccessful".to_string()),
            }),
            _ => Ok(VerificationResponse {
                token: Some(format!(
                    "Authorization: Bearer ey{}{}",
                    request.number,
                    chrono::offset::Utc::now().timestamp(),
                )),
                error: None,
            }),
        }
    }

    // returns rankings of carrier validation rates
    pub fn get_provider_rank(&self) -> RankResponse {
        RankResponse {
            rank: self.repo.get_provider_rank(),
        }
    }
}

// used for BestBalancer and RoudRobinBalancer
pub trait Balancer: Send + Sync {
    fn next_idx(&mut self, carrier_len: usize) -> usize;
}

#[derive(Debug)]
pub struct RoundRobinBalancer {
    cur_idx: Arc<RwLock<usize>>,
}

impl RoundRobinBalancer {
    pub fn new() -> RoundRobinBalancer {
        Self {
            cur_idx: Arc::new(RwLock::new(0)),
        }
    }
}

impl Balancer for RoundRobinBalancer {
    fn next_idx(&mut self, carrier_len: usize) -> usize {
        let mut ci = self.cur_idx.write().unwrap();
        let idx = *ci;
        // let idx = ci.into();
        // rotate to next index
        *ci = (*ci + 1) % carrier_len;
        idx
    }
}

// unwrap_request attempts
pub fn unwrap_request(request: &Request) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut body = match request.data() {
        Some(b) => b,
        None => return buffer,
    };
    match body.read_to_end(&mut buffer) {
        Err(e) => return Vec::from(e.to_string()),
        Ok(_) => (),
    };
    buffer
}
