use crate::provider::*;
use crate::repo::*;
use anyhow::{anyhow, Error};
use argh::FromArgs;
use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use rouille::Request;
use serde::{Deserialize, Serialize};
use std::io::Read;
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

    /// the port that the telecom verification service runs on [ default: 5000 ]
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
            "rr" | "round_robin" => Ok(Self::RoundRobin),
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

pub struct VerificationServer<'a> {
    carriers: Vec<Box<dyn TelecomProvider<'a>>>,
    balancer: Box<dyn Balancer>,
    repo: Box<dyn VerificationRepo>,
}

impl<'a> VerificationServer<'a> {
    pub fn new(
        client_mode: BalancerType,
        carriers: Vec<Box<dyn TelecomProvider<'a>>>,
        repo: Box<dyn VerificationRepo>,
    ) -> Self {
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
        &'a mut self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, Error> {
        let carrier = match self.carriers.get(self.balancer.next_idx(&self.carriers)) {
            Some(c) => c,
            None => {
                return Ok(VerificationResponse {
                    token: None,
                    error: Some("no carriers found".to_string()),
                })
            }
        };
        let entry = carrier.verify(&request.number);
        self.repo.store_attempt(entry.clone())?;
        match entry.step {
            VerificationStep::Unreachable => Ok(VerificationResponse {
                token: None,
                error: Some("verification unsuccessful".to_string()),
            }),
            _ => Ok(VerificationResponse {
                token: Some("verification_token_test".to_string()),
                error: None,
            }),
        }
    }
}

// used for BestBalancer and RoudRobinBalancer
pub trait Balancer {
    fn next_idx(&mut self, carriers: &Vec<Box<dyn TelecomProvider>>) -> usize;
}

#[derive(Debug)]
pub struct RoundRobinBalancer {
    cur_idx: usize,
}

impl RoundRobinBalancer {
    pub fn new() -> RoundRobinBalancer {
        Self { cur_idx: 0 }
    }
}

impl Balancer for RoundRobinBalancer {
    fn next_idx(&mut self, carriers: &Vec<Box<dyn TelecomProvider>>) -> usize {
        let idx = self.cur_idx;
        // rotate to next index
        self.cur_idx = (self.cur_idx + 1) % carriers.len();
        idx
    }
}

// unwrap_request attemps
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
