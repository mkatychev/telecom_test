use crate::provider::*;
use crate::repo::*;
use anyhow::{anyhow, Error};
use argh::FromArgs;
use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use rouille::{Request, RequestBody, Response};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

mod provider;
mod repo;

/// Top-level command.
#[derive(FromArgs, PartialEq, Debug)]
pub struct Command {
    /// strategy in selecting what telecom provider handles a verification attempt
    #[argh(option)]
    balancer: BalancerType,

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
    carriers: Vec<Box<dyn TelecomProvider>>,
    balancer: Box<dyn Balancer<'a>>,
    repo: Box<dyn VerificationRepo>,
}

impl<'a> VerificationServer<'a> {
    fn new(
        client_mode: BalancerType,
        carriers: Vec<Box<dyn TelecomProvider>>,
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
    // fn handle_request(&VerificationRequest
}

// used for BestBalancer and RoudRobinBalancer
pub trait Balancer<'a> {
    fn next(
        &self,
        carriers: &'a Vec<Box<dyn TelecomProvider>>,
    ) -> Option<&'a Box<dyn TelecomProvider>>;
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

impl<'a> Balancer<'a> for RoundRobinBalancer {
    fn next(
        &self,
        carriers: &'a Vec<Box<dyn TelecomProvider>>,
    ) -> Option<&'a Box<dyn TelecomProvider>> {
        let mut ci = self.cur_idx.write().unwrap();
        let s = carriers.get(*ci);
        // rotate to next index
        *ci = (*ci + 1) % carriers.len();
        s.clone()
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
