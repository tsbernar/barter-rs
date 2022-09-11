use super::order::{
    Cancelled, Order, RequestCancel, RequestOpen
};
use barter_data::model::MarketEvent;

pub trait IndicatorUpdater {
    fn update_indicators(&mut self, market: &MarketEvent);
}


// Todo:
//  - Name clashes with OrderGenerator<State>
//  - Do I want two seperate states, one for generate_cancel(), one for generate_orders()?
pub trait OrderGenerator {
    fn generate_cancels(&self) -> Option<Vec<Order<RequestCancel>>>;
    fn generate_orders(&self) -> Option<Vec<Order<RequestOpen>>>;
}


// Todo: What does the Strategy do?
// - Updates Indicators
// - Analyses Indicators, in conjunction with Statistics, Positions, and Orders
// - Based on analysis, generates optional Order<Request>
// - Allocates Order<Request>
// - Decides Order<Request> OrderKind