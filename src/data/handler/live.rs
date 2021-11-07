use crate::data::error::DataError;
use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::data::market::MarketEvent;
use barter_data::client::binance::Binance;
use barter_data::client::{ClientConfig, ClientName as ExchangeName};
use barter_data::model::{Candle, MarketData};
use barter_data::ExchangeClient;
use log::debug;
use std::sync::mpsc::{channel, Receiver};
use chrono::Utc;
use serde::Deserialize;
use tokio_stream::StreamExt;
use uuid::Uuid;

// Todo:
//  - Ensure there is a proper pattern for .expect() in barter componenet new() methods
//     '--> Should LiveCandleHandler::new() return a Result<>?
//            '--> See what makes sense in barter-execution / backtester

/// Configuration for constructing a [LiveCandleHandler] via the new() constructor method.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub rate_limit_per_minute: u64,
    pub exchange: ExchangeName,
    pub symbol: String,
    pub interval: String,
}

/// [MarketEvent] data handler that consumes a live [UnboundedReceiverStream] of [Candle]s. Implements
/// [Continuer] & [MarketGenerator].
pub struct LiveCandleHandler {
    pub exchange: ExchangeName,
    pub symbol: String,
    pub interval: String,
    candle_rx: Receiver<Candle>,
}

impl Continuer for LiveCandleHandler {
    fn can_continue(&mut self) -> Continuation {
        Continuation::Continue
    }
}

impl MarketGenerator for LiveCandleHandler {
    fn generate_market(&mut self) -> Option<MarketEvent> {
        // Consume next candle
        let candle = self.candle_rx.recv().unwrap();

        Some(MarketEvent {
            event_type: MarketEvent::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: format!("{:?}", self.exchange.clone()),
            symbol: self.symbol.clone(),
            data: MarketData::Candle(candle),
        })
    }
}

impl LiveCandleHandler {
    /// Constructs a new [LiveCandleHandler] component using the provided [Config] struct, as well
    /// as a oneshot::[Receiver] for receiving [TerminateCommand]s.
    pub async fn new(cfg: &Config) -> Self {
        // Determine ExchangeClient type & construct
        let mut exchange_client = match cfg.exchange {
            ExchangeName::Binance => Binance::new(ClientConfig {
                rate_limit_per_minute: cfg.rate_limit_per_minute,
            }),
        }
            .await
            .expect("Failed to construct exchange Client instance");

        // Subscribe to candle stream via exchange Client
        let mut candle_stream = exchange_client
            .consume_candles(cfg.symbol.clone(), &cfg.interval)
            .await
            .expect("Failed to consume_candles for via exchange Client instance");

        // Spawn Tokio task to async consume_candles from Client and transmit to a sync candle_rx
        let (candle_tx, candle_rx) = channel();
        tokio::spawn(async move {
            // Send any received Candles from Client to the LiveCandleHandler candle_rx
            if let Some(candle) = candle_stream.next().await {
                if candle_tx.send(candle).is_err() {
                    debug!("Receiver for exchange Candles has been dropped - closing channel");
                    return
                }
            }
        });

        Self {
            exchange: cfg.exchange.clone(),
            symbol: cfg.symbol.clone(),
            interval: cfg.interval.clone(),
            candle_rx,
        }
    }

    /// Returns a [LiveCandleHandlerBuilder] instance.
    pub fn builder() -> LiveCandleHandlerBuilder {
        LiveCandleHandlerBuilder::new()
    }
}

/// Builder to construct [LiveCandleHandler] instances.
#[derive(Debug, Default)]
pub struct LiveCandleHandlerBuilder {
    pub exchange: Option<ExchangeName>,
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub candle_rx: Option<Receiver<Candle>>,
}

impl LiveCandleHandlerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn exchange(self, value: ExchangeName) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }

    pub fn symbol(self, value: String) -> Self {
        Self {
            symbol: Some(value),
            ..self
        }
    }

    pub fn interval(self, value: String) -> Self {
        Self {
            interval: Some(value),
            ..self
        }
    }

    pub fn candle_rx(self, value: Receiver<Candle>) -> Self {
        Self {
            candle_rx: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<LiveCandleHandler, DataError> {
        let exchange = self.exchange.ok_or(DataError::BuilderIncomplete)?;
        let symbol = self.symbol.ok_or(DataError::BuilderIncomplete)?;
        let interval = self.interval.ok_or(DataError::BuilderIncomplete)?;
        let candle_rx = self.candle_rx.ok_or(DataError::BuilderIncomplete)?;

        Ok(LiveCandleHandler {
            exchange,
            symbol,
            interval,
            candle_rx,
        })
    }
}