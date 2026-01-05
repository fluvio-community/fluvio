#![allow(unused)]

use futures_util::StreamExt;
use async_channel::Receiver;
use crate::{
    CheckResult, CheckResults, CheckStatus, CheckSuggestion,
    render::{ProgressRenderedText, ProgressRenderer},
};

const ISSUE_URL: &str = "https://github.com/fluvio-community/fluvio/issues/new/choose";
