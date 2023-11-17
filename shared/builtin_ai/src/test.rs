use oort_api::prelude::*;

pub struct Ship {}

#[allow(clippy::empty_loop)]
impl Ship {
    pub fn new() -> Ship {
        Ship {}
    }

    pub fn tick(&mut self) {
        let testcase = oort_api::sys::getenv("TESTCASE").unwrap_or("none");
        match testcase {
            "scenario_name" => debug!("Scenario: {}", scenario_name()),
            "world_size" => debug!("World size: {}", world_size()),
            "id" => debug!("ID: {}", id()),
            "panic" => panic!("Panic!"),
            "infinite_loop" => loop {},
            _ => debug!("Unknown testcase: {:?}", testcase),
        }
    }
}
