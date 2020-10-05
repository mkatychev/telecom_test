use crate::repo::{VerificationEntry, VerificationStep};
use anyhow::{anyhow, Error};
use rand::Rng;

// TelecomProvider encapsulates the verification flow between a telecom provider
//
// For this scenario there is an assumption that a TelecomProvider handles not only the SMS/Voice
// request to the provide but also the webhook that listens to a user's valid submission of the 6
// digit string and verification token
pub trait TelecomProvider: Send + Sync {
    fn send_sms(&self, number: &String) -> bool;
    fn send_voice(&self, number: &String) -> bool;
    fn verify(&self, number: &String) -> VerificationEntry;
    fn get_name(&self) -> String;
}

pub struct MockTelecomProvider {
    name: String,
    // percentage based likelyhood of success
    chance_sms: u8,
    chance_voice: u8,
}

impl MockTelecomProvider {
    pub fn new<T: ToString>(name: T, chance_sms: u8, chance_voice: u8) -> Result<Self, Error> {
        if chance_sms > 100 || chance_voice > 100 {
            return Err(anyhow!("probability must be a number between 0 and 100"));
        }

        Ok(Self {
            name: name.to_string(),
            chance_sms,
            chance_voice,
        })
    }
}

impl TelecomProvider for MockTelecomProvider {
    // return a probability likelyhood of verification success,
    fn send_sms(&self, _number: &String) -> bool {
        let num = rand::thread_rng().gen_range(0, 100);
        num <= self.chance_sms
    }
    fn send_voice(&self, _number: &String) -> bool {
        let num = rand::thread_rng().gen_range(0, 100);
        num <= self.chance_voice
    }

    // step through the steps outlined in VerificationStep with each having an independent chance
    // of success, returning the first verification attempt that returns true
    fn verify(&self, number: &String) -> VerificationEntry
    where
        Self: Send + Sync,
    {
        let rng_verification_step: VerificationStep = match () {
            _ if self.send_sms(number) => VerificationStep::FirstSMS,
            _ if self.send_sms(number) => VerificationStep::SecondSMS,
            _ if self.send_voice(number) => VerificationStep::FirstTextToSpeech,
            _ if self.send_voice(number) => VerificationStep::SecondTextToSpeech,
            _ => VerificationStep::Unreachable,
        };

        VerificationEntry {
            carrier: self.name.clone(),
            number: number.clone(),
            time: chrono::offset::Utc::now(),
            step: rng_verification_step,
        }
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }
}
