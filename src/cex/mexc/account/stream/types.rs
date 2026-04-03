#[derive(Debug, Clone)]
pub struct MexcBalance {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
    pub available: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct MexcAccountBalanceSnapshot {
    pub account_type: Option<String>,
    pub balances: Vec<MexcBalance>,
    pub permissions: Vec<String>,
    pub can_trade: bool,
    pub can_withdraw: bool,
    pub can_deposit: bool,
    pub update_time: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct MexcSpotOrderUpdate {
    pub symbol: String,
    pub channel: String,
    pub send_time: u64,
    pub order_id: String,
    pub client_id: String,
    pub price: f64,
    pub quantity: f64,
    pub amount: f64,
    pub avg_price: f64,
    pub order_type: MexcOrderType,
    pub side: MexcTradeSide,
    pub remain_amount: f64,
    pub remain_quantity: f64,
    pub last_deal_quantity: Option<f64>,
    pub cumulative_quantity: f64,
    pub cumulative_amount: f64,
    pub status: MexcOrderStatus,
    pub create_time: u64,
}

#[derive(Debug, Clone)]
pub struct MexcSpotOrderDisplay {
    pub trading_pair: String,
    pub time: u64,
    pub order_type: String,
    pub side: String,
    pub event_action: String,
    pub average_filled_price: f64,
    pub price: String,
    pub executed: f64,
    pub quantity: Option<f64>,
    pub order_amount: Option<f64>,
    pub total: f64,
    pub status: String,
}

impl MexcSpotOrderUpdate {
    pub fn filled_quantity(&self) -> f64 {
        if self.cumulative_quantity > 0.0 {
            return self.cumulative_quantity;
        }
        self.last_deal_quantity.unwrap_or(0.0)
    }

    pub fn display_quantity(&self) -> f64 {
        if self.quantity > 0.0 {
            return self.quantity;
        }
        self.filled_quantity()
    }

    pub fn to_display(&self) -> MexcSpotOrderDisplay {
        let executed = self.filled_quantity();
        let quantity = if self.quantity > 0.0 {
            Some(self.quantity)
        } else {
            None
        };
        let order_amount = if self.amount > 0.0 {
            Some(self.amount)
        } else {
            None
        };

        MexcSpotOrderDisplay {
            trading_pair: self.symbol.clone(),
            time: if self.send_time > 0 {
                self.send_time
            } else {
                self.create_time
            },
            order_type: self.order_type.label().to_string(),
            side: self.side.label().to_string(),
            event_action: self.status.event_action(self).to_string(),
            average_filled_price: self.avg_price,
            price: if matches!(self.order_type, MexcOrderType::Market) {
                "Market".to_string()
            } else {
                self.price.to_string()
            },
            executed,
            quantity,
            order_amount,
            total: if self.cumulative_amount > 0.0 {
                self.cumulative_amount
            } else {
                self.amount
            },
            status: self.status.label().to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MexcOrderType {
    Limit,
    PostOnly,
    ImmediateOrCancel,
    FillOrKill,
    Market,
    StopLossTakeProfit,
    Unknown(i32),
}

impl From<i32> for MexcOrderType {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Limit,
            2 => Self::PostOnly,
            3 => Self::ImmediateOrCancel,
            4 => Self::FillOrKill,
            5 => Self::Market,
            100 => Self::StopLossTakeProfit,
            other => Self::Unknown(other),
        }
    }
}

impl MexcOrderType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Limit => "Limit",
            Self::PostOnly => "Post Only",
            Self::ImmediateOrCancel => "IOC",
            Self::FillOrKill => "FOK",
            Self::Market => "Market",
            Self::StopLossTakeProfit => "TP/SL",
            Self::Unknown(_) => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MexcTradeSide {
    Buy,
    Sell,
    Unknown(i32),
}

impl From<i32> for MexcTradeSide {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Buy,
            2 => Self::Sell,
            other => Self::Unknown(other),
        }
    }
}

impl MexcTradeSide {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Buy => "Buy",
            Self::Sell => "Sell",
            Self::Unknown(_) => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MexcOrderStatus {
    New,
    Filled,
    PartiallyFilled,
    Cancelled,
    PartiallyCancelled,
    Unknown(i32),
}

impl From<i32> for MexcOrderStatus {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::New,
            2 => Self::Filled,
            3 => Self::PartiallyFilled,
            4 => Self::Cancelled,
            5 => Self::PartiallyCancelled,
            other => Self::Unknown(other),
        }
    }
}

impl MexcOrderStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::New => "New",
            Self::Filled => "Filled",
            Self::PartiallyFilled => "Partially Filled",
            Self::Cancelled => "Cancelled",
            Self::PartiallyCancelled => "Partially Cancelled",
            Self::Unknown(_) => "Unknown",
        }
    }

    pub fn event_action(&self, order: &MexcSpotOrderUpdate) -> &'static str {
        match self {
            Self::New => {
                if matches!(order.order_type, MexcOrderType::Limit) {
                    "Created"
                } else {
                    "Opened"
                }
            }
            Self::Filled => "Filled",
            Self::PartiallyFilled => "Partially Filled",
            Self::Cancelled => "Cancelled",
            Self::PartiallyCancelled => "Partially Cancelled",
            Self::Unknown(_) => "Updated",
        }
    }
}
