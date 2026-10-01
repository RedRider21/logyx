// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Errore comune del compilatore (lessicale, sintattico, di tipo).

use std::fmt;

#[derive(Debug, Clone)]
pub struct LogyxError {
    pub message: String,
}

impl LogyxError {
    pub fn new(message: impl Into<String>) -> Self {
        LogyxError { message: message.into() }
    }
}

impl fmt::Display for LogyxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LogyxError {}
