use core::panic;
use std::collections::HashMap;

const DEFAULT_INVENTORY_CAPACITY: u32 = 100;
const DEFAULT_INVENTORY_AMOUNT: u32 = 10;
const DEFAULT_BALANCE: i32 = 100;

type CommodityName = String;
type ProductionStrategyName = String;

fn main() {
    println!("Hello, world!");
    let mut market = Market::default();

    market
        .add_production_strategy("farmer")
        .add_input("water", 1)
        .add_output("food", 1)
        .duration(3);

    market
        .add_production_strategy("water-source")
        .add_output("water", 1);

    market.add_agent().add_production_strategy("farmer");

    market.add_agent().add_production_strategy("water-source");

    market
        .add_agent()
        .add_production_strategy("farmer")
        .add_production_strategy("water-source");

    println!("{:#?}", market);
    println!("===================");
    market.run_production_step();
    println!("{:#?}", market.agents);
    println!("===================");
    market.run_production_step();
    println!("{:#?}", market.agents);
}

#[derive(Default, Debug)]
pub struct Market {
    pub buy_offers: Vec<TradeOffer>,
    pub sell_offers: Vec<TradeOffer>,
    pub agents: Vec<Agent>,
    pub production_strategies: HashMap<ProductionStrategyName, ProductionStrategy>,
}

impl Market {
    pub fn add_production_strategy(&mut self, name: &str) -> &mut ProductionStrategy {
        let production_strategy = ProductionStrategy::new();

        self.production_strategies
            .insert(name.to_string(), production_strategy);

        self.production_strategies.get_mut(name).unwrap()
    }

    pub fn add_agent(&mut self) -> MarketAgentBuilder {
        self.agents.push(Agent::new());

        MarketAgentBuilder {
            production_strategies: &self.production_strategies,
            agent: self.agents.last_mut().unwrap(),
        }
    }

    pub fn get_agents_mut(&mut self) -> impl Iterator<Item = MarketAgentBuilder> {
        self.agents.iter_mut().map(|agent| -> MarketAgentBuilder {
            MarketAgentBuilder {
                agent,
                production_strategies: &self.production_strategies,
            }
        })
    }

    pub fn run_production_step(&mut self) {
        self.agents
            .iter_mut()
            .for_each(|agent| agent.run_production_step(&self.production_strategies))
    }
}

pub struct MarketAgentBuilder<'a> {
    production_strategies: &'a HashMap<String, ProductionStrategy>,
    agent: &'a mut Agent,
}

impl MarketAgentBuilder<'_> {
    pub fn add_production_strategy(&mut self, production_strategy_name: &str) -> &mut Self {
        let production_strategy = self
            .production_strategies
            .get(production_strategy_name)
            .unwrap();

        let production_requirements = production_strategy
            .inputs
            .iter()
            .chain(production_strategy.outputs.iter());

        self.agent
            .producers
            .push(Producer::new(production_strategy_name.to_string()));

        production_requirements.for_each(|production_requirement| {
            self.agent.inventories.insert(
                production_requirement.commodity_name.clone(),
                Inventory::new(DEFAULT_INVENTORY_CAPACITY),
            );
        });

        self
    }
}

#[derive(Default, Debug)]
pub struct Agent {
    pub inventories: HashMap<CommodityName, Inventory>,
    pub producers: Vec<Producer>,
    pub balance: i32,
}

#[derive(Debug)]
pub struct Producer {
    production_strategy_name: ProductionStrategyName,
    progress: u32,
}

impl Producer {
    fn new(production_strategy_name: ProductionStrategyName) -> Self {
        Self {
            production_strategy_name,
            progress: 0,
        }
    }
}

impl Agent {
    fn new() -> Self {
        Self {
            balance: DEFAULT_BALANCE,
            ..Self::default()
        }
    }

    pub fn inventory_amount(&self, commodity_name: &CommodityName) -> u32 {
        if let Some(inventory) = self.inventories.get(commodity_name) {
            inventory.amount
        } else {
            0
        }
    }

    pub fn inventory_capacity(&self, commodity_name: &CommodityName) -> u32 {
        if let Some(inventory) = self.inventories.get(commodity_name) {
            inventory.capacity
        } else {
            0
        }
    }

    pub fn run_production_step(
        &mut self,
        production_strategies: &HashMap<ProductionStrategyName, ProductionStrategy>,
    ) {
        let producers = self.producers.iter_mut();
        let inventories = &mut self.inventories;

        producers.for_each(|producer| {
            let production_strategy = production_strategies
                .get(&producer.production_strategy_name)
                .unwrap();

            // Start production
            if producer.progress == 0 {
                let inputs_are_satisfied =
                    production_strategy
                        .inputs
                        .iter()
                        .all(|production_requirement| {
                            let inventory = inventories
                                .get(&production_requirement.commodity_name)
                                .unwrap();

                            inventory.unreserved() >= production_requirement.amount
                        });

                // Reserve required inputs
                if inputs_are_satisfied {
                    production_strategy
                        .inputs
                        .iter()
                        .for_each(|production_requirement| {
                            let inventory = inventories
                                .get_mut(&production_requirement.commodity_name)
                                .unwrap();

                            inventory.reserve(production_requirement.amount);
                        });

                    producer.progress += 1;
                }
            } else if production_strategy.duration <= producer.progress {
                let has_room_for_outputs =
                    production_strategy
                        .outputs
                        .iter()
                        .all(|production_requirement| {
                            let inventory = inventories
                                .get(&production_requirement.commodity_name)
                                .unwrap();
                            inventory.free() >= production_requirement.amount
                        });

                if has_room_for_outputs {
                    production_strategy
                        .inputs
                        .iter()
                        .for_each(|production_requirement| {
                            let inventory = inventories
                                .get_mut(&production_requirement.commodity_name)
                                .unwrap();

                            inventory.remove(production_requirement.amount);
                            inventory.unreserve(production_requirement.amount);
                        });

                    production_strategy
                        .outputs
                        .iter()
                        .for_each(|production_requirement| {
                            let inventory = inventories
                                .get_mut(&production_requirement.commodity_name)
                                .unwrap();

                            inventory.add(production_requirement.amount);
                        });

                    producer.progress = 0;
                }
            } else {
                producer.progress += 1;
            }
        })
    }
}

#[derive(Debug)]
pub struct Inventory {
    pub capacity: u32,
    pub amount: u32,
    pub reserved: u32,
}

impl Inventory {
    fn new(capacity: u32) -> Self {
        Self {
            amount: DEFAULT_INVENTORY_AMOUNT,
            capacity,
            reserved: 0,
        }
    }

    fn add(&mut self, amount: u32) {
        if (amount > self.free()) {
            panic!("Tried to add more than there is room for")
        }

        self.amount += amount;
    }

    fn remove(&mut self, amount: u32) {
        if (amount > self.free()) {
            panic!("Tried to remove more than is available")
        }

        self.amount -= amount;
    }

    fn free(&self) -> u32 {
        self.capacity - self.amount
    }

    fn unreserved(&self) -> u32 {
        self.amount - self.reserved
    }

    fn reserve(&mut self, amount: u32) {
        if amount > self.unreserved() {
            panic!("Tried to reserve more than is available")
        }

        self.reserved += amount;
    }

    fn unreserve(&mut self, amount: u32) {
        if amount > self.unreserved() {
            panic!("Tried to unreserve more than is reserved")
        }

        self.reserved -= amount;
    }
}

pub fn get_inventory_amount(
    inventories: &HashMap<CommodityName, Inventory>,
    commodity_name: &CommodityName,
) -> u32 {
    if let Some(inventory) = inventories.get(commodity_name) {
        inventory.amount
    } else {
        0
    }
}

pub fn get_inventory_capacity(
    inventories: &HashMap<CommodityName, Inventory>,
    commodity_name: &CommodityName,
) -> u32 {
    if let Some(inventory) = inventories.get(commodity_name) {
        inventory.capacity
    } else {
        0
    }
}

#[derive(Debug)]
pub struct ProductionRequirement {
    pub commodity_name: CommodityName,
    pub amount: u32,
}

impl ProductionRequirement {
    pub fn new(commodity_name: CommodityName, amount: u32) -> Self {
        Self {
            commodity_name,
            amount,
        }
    }
}

#[derive(Default, Debug)]
pub struct ProductionStrategy {
    pub inputs: Vec<ProductionRequirement>,
    pub outputs: Vec<ProductionRequirement>,
    pub duration: u32,
}

impl ProductionStrategy {
    fn new() -> Self {
        ProductionStrategy {
            duration: 1,
            ..ProductionStrategy::default()
        }
    }

    fn add_input(&mut self, commodity_name: &str, amount: u32) -> &mut Self {
        self.inputs.push(ProductionRequirement::new(
            commodity_name.to_string(),
            amount,
        ));
        self
    }

    fn add_output(&mut self, commodity_name: &str, amount: u32) -> &mut Self {
        self.outputs.push(ProductionRequirement::new(
            commodity_name.to_string(),
            amount,
        ));
        self
    }

    fn duration(&mut self, duration: u32) -> &mut Self {
        self.duration = duration;
        self
    }
}

#[derive(Debug)]
pub struct TradeOffer {
    pub commodity_name: String,
    pub ideal_amount: i32,
    pub max_amount: i32,
    pub price: i32,
}

mod tests {
    use crate::Market;

    #[test]
    fn production_step() {
        let mut market = Market::default();

        market
            .add_production_strategy("farmer")
            .add_input("water", 1)
            .add_output("food", 1)
            .duration(1);

        market.add_agent().add_production_strategy("farmer");

        {
            let agent = market.agents.last().unwrap();
            assert_eq!(agent.producers.last().unwrap().progress, 0);
            assert_eq!(agent.inventories.get("water").unwrap().amount, 10);
            assert_eq!(agent.inventories.get("food").unwrap().amount, 10);
        }

        market.run_production_step();

        {
            let agent = market.agents.last().unwrap();
            assert_eq!(agent.producers.last().unwrap().progress, 1);
            assert_eq!(agent.inventories.get("water").unwrap().amount, 10);
            assert_eq!(agent.inventories.get("food").unwrap().amount, 10);
        }

        market.run_production_step();

        {
            let agent = market.agents.last().unwrap();
            assert_eq!(agent.producers.last().unwrap().progress, 0);
            assert_eq!(agent.inventories.get("water").unwrap().amount, 9);
            assert_eq!(agent.inventories.get("food").unwrap().amount, 11);
        }
    }
}
