use anyhow::{anyhow, Error};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

pub trait VerificationRepo {
    fn store_attempt(&mut self, entry: VerificationEntry) -> Result<(), Error>;
    fn get_provider_rank(&self) -> Vec<(String, f32)>;
}

#[derive(Clone)]
pub struct VerificationEntry {
    pub carrier: String,
    pub number: String,
    pub time: DateTime<Utc>,
    pub step: VerificationStep,
}

/// represents outcome of last verification attempt for a given phone number, values 1-5 represent:
/// 1. verified on first SMS from telecom provider
/// 2. verified on second SMS from telecom provider
/// 3. verified on first text to speech call from telecom provider
/// 4. verified on second text to speech call from telecom provider
/// 5.  phone number was unreachable from telecom provider
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum VerificationStep {
    FirstSMS,
    SecondSMS,
    FirstTextToSpeech,
    SecondTextToSpeech,
    Unreachable,
}

// in-memory implementation of VerificationEntry trait
pub struct VerificationKeeper {
    entries: Vec<VerificationEntry>,
    step_weights: HashMap<VerificationStep, u32>,
}

impl VerificationKeeper {
    pub fn new(step_values: [u32; 5]) -> Result<Self, Error> {
        let mut sorted_steps = step_values.clone();
        sorted_steps.sort();
        if step_values != sorted_steps {
            return Err(anyhow!(
                "step_values must be provided in ascending sequence"
            ));
        }
        let mut step_weights = HashMap::new();

        // assign weighted value to the corresponding VerificationStep
        step_weights.insert(VerificationStep::FirstSMS, step_values[0]);
        step_weights.insert(VerificationStep::SecondSMS, step_values[1]);
        step_weights.insert(VerificationStep::FirstTextToSpeech, step_values[2]);
        step_weights.insert(VerificationStep::SecondTextToSpeech, step_values[3]);
        step_weights.insert(VerificationStep::Unreachable, step_values[4]);

        Ok(Self {
            entries: Vec::new(),
            step_weights: step_weights,
        })
    }

    // get_weighted_avg returns the weighted value of a particular carrier's verification attempts
    fn get_weighted_avg(&self, attempts: &Vec<VerificationStep>) -> f32 {
        let total_attempts = &attempts.len();
        let weighted_sum: u32 = attempts.into_iter().map(|s| self.step_weights[&s]).sum();
        weighted_sum as f32 / *total_attempts as f32
    }
}

impl VerificationRepo for VerificationKeeper {
    // store_attempt attempts to store a VerificationEntry in the keeper struct
    // Error would be returned in the a failed transaction for a production DB
    fn store_attempt(&mut self, entry: VerificationEntry) -> Result<(), Error> {
        self.entries.push(entry);
        Ok(())
    }

    // return the telecom providers and their corresponding weighted average
    fn get_provider_rank(&self) -> Vec<(String, f32)> {
        let mut by_carrier: HashMap<String, Vec<VerificationStep>> = HashMap::new();
        for entry in self.entries.iter() {
            match by_carrier.get_mut(&entry.carrier) {
                Some(v) => v.push(entry.step),
                None => {
                    by_carrier.insert(entry.carrier.clone(), vec![entry.step]);
                }
            }
        }

        let mut rank = by_carrier
            .iter()
            .map(|(k, v)| (k.clone(), self.get_weighted_avg(v)))
            .collect::<Vec<(String, f32)>>();

        rank.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // sort by weighted value
        rank
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_keeper() {
        let mut keeper =
            VerificationKeeper::new([1, 2, 3, 4, 5]).expect("failed to create new keeper");
        keeper
            .store_attempt(VerificationEntry {
                carrier: "carrier_1".to_owned(),
                number: "0177".to_owned(),
                time: chrono::offset::Utc::now(),
                step: VerificationStep::FirstSMS,
            })
            .unwrap();

        assert_eq!(
            keeper.get_provider_rank(),
            vec![("carrier_1".to_owned(), 1.0)]
        );

        keeper
            .store_attempt(VerificationEntry {
                carrier: "carrier_1".to_owned(),
                number: "0178".to_owned(),
                time: chrono::offset::Utc::now(),
                step: VerificationStep::Unreachable,
            })
            .unwrap();

        keeper
            .store_attempt(VerificationEntry {
                carrier: "carrier_2".to_owned(),
                number: "0179".to_owned(),
                time: chrono::offset::Utc::now(),
                step: VerificationStep::FirstSMS,
            })
            .unwrap();

        keeper
            .store_attempt(VerificationEntry {
                carrier: "carrier_2".to_owned(),
                number: "0180".to_owned(),
                time: chrono::offset::Utc::now(),
                step: VerificationStep::SecondSMS,
            })
            .unwrap();

        assert_eq!(
            keeper.get_provider_rank(),
            vec![("carrier_2".to_owned(), 1.5), ("carrier_1".to_owned(), 3.0)]
        );
    }
}
