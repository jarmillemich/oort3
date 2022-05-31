use crate::rng::{new_rng, SeededRng};
use crate::ship::{asteroid, fighter, target, ShipHandle};
use crate::simulation::{Line, Simulation, WORLD_SIZE};
use crate::{bullet, collision, ship};
use bullet::BulletData;
use nalgebra::{Point2, Rotation2, Translation2};
use rand::seq::SliceRandom;
use rand::Rng;
use rapier2d_f64::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use Status::Running;

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Copy, Clone)]
pub enum Status {
    Running,
    Victory { team: i32 },
    Failed,
}

fn check_victory(sim: &Simulation) -> Status {
    let mut alive_teams: HashSet<i32> = HashSet::new();
    for &handle in sim.ships.iter() {
        let ship = sim.ship(handle);
        if ship.data().class == ship::ShipClass::Missile {
            continue;
        }
        alive_teams.insert(ship.data().team);
    }
    if alive_teams.is_empty() {
        Status::Victory { team: 0 }
    } else if alive_teams.len() == 1 {
        Status::Victory {
            team: *alive_teams.iter().next().unwrap(),
        }
    } else {
        Status::Running
    }
}

fn check_tutorial_victory(sim: &Simulation) -> Status {
    match check_victory(sim) {
        x @ Status::Victory { team: 0 } => x,
        Status::Victory { .. } => Status::Failed,
        x => x,
    }
}

pub trait Scenario {
    fn name(&self) -> String;

    fn init(&mut self, sim: &mut Simulation, seed: u32);

    fn tick(&mut self, _: &mut Simulation) {}

    fn status(&self, _: &Simulation) -> Status {
        Running
    }

    fn initial_code(&self) -> String {
        "".to_string()
    }

    fn solution(&self) -> String {
        "".to_string()
    }

    fn next_scenario(&self) -> Option<String> {
        None
    }

    fn lines(&self) -> Vec<Line> {
        vec![]
    }
}

pub fn add_walls(sim: &mut Simulation) {
    let mut make_edge = |x: f64, y: f64, a: f64| {
        let edge_length = WORLD_SIZE as f64;
        let edge_width = 10.0;
        let rigid_body = RigidBodyBuilder::new_static()
            .translation(vector![x, y])
            .rotation(a)
            .build();
        let body_handle = sim.bodies.insert(rigid_body);
        let collider = ColliderBuilder::cuboid(edge_length / 2.0, edge_width / 2.0)
            .restitution(1.0)
            .collision_groups(collision::wall_interaction_groups())
            .build();
        sim.colliders
            .insert_with_parent(collider, body_handle, &mut sim.bodies);
    };
    make_edge(0.0, WORLD_SIZE / 2.0, 0.0);
    make_edge(0.0, -WORLD_SIZE / 2.0, std::f64::consts::PI);
    make_edge(WORLD_SIZE / 2.0, 0.0, std::f64::consts::PI / 2.0);
    make_edge(-WORLD_SIZE / 2.0, 0.0, 3.0 * std::f64::consts::PI / 2.0);
}

pub fn load(name: &str) -> Box<dyn Scenario> {
    let scenario: Box<dyn Scenario> = match name {
        // Testing
        "test" => Box::new(TestScenario {}),
        "basic" => Box::new(BasicScenario {}),
        "gunnery" => Box::new(GunneryScenario {}),
        "asteroid-stress" => Box::new(AsteroidStressScenario {}),
        "bullet-stress" => Box::new(BulletStressScenario {}),
        "welcome" => Box::new(WelcomeScenario::new()),
        // Tutorials
        "tutorial01" => Box::new(Tutorial01 {}),
        "tutorial02" => Box::new(Tutorial02::new()),
        "tutorial03" => Box::new(Tutorial03::new()),
        "tutorial04" => Box::new(Tutorial04::new()),
        "tutorial05" => Box::new(Tutorial05::new()),
        "tutorial06" => Box::new(Tutorial06::new()),
        "tutorial07" => Box::new(Tutorial07::new()),
        "tutorial08" => Box::new(Tutorial08::new()),
        "tutorial09" => Box::new(Tutorial09::new()),
        // Tournament
        "duel" => Box::new(Duel::new()),
        _ => panic!("Unknown scenario"),
    };
    assert_eq!(scenario.name(), name);
    scenario
}

pub fn list() -> Vec<String> {
    vec![
        "welcome",
        "tutorial01",
        "tutorial02",
        "tutorial03",
        "tutorial04",
        "tutorial05",
        "tutorial06",
        "tutorial07",
        "tutorial08",
        "tutorial09",
        "gunnery",
        "duel",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect()
}

struct TestScenario {}

impl Scenario for TestScenario {
    fn name(&self) -> String {
        "test".into()
    }

    fn init(&mut self, _sim: &mut Simulation, _seed: u32) {}
}

struct BasicScenario {}

impl Scenario for BasicScenario {
    fn name(&self) -> String {
        "basic".into()
    }

    fn init(&mut self, sim: &mut Simulation, _seed: u32) {
        add_walls(sim);
        ship::create(sim, -100.0, 0.0, 0.0, 0.0, 0.0, fighter(0));
        ship::create(sim, 100.0, 0.0, 0.0, 0.0, std::f64::consts::PI, fighter(1));
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_victory(sim)
    }
}

struct GunneryScenario {}

impl Scenario for GunneryScenario {
    fn name(&self) -> String {
        "gunnery".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        add_walls(sim);
        ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, fighter(0));
        let mut rng = new_rng(seed);
        for _ in 0..4 {
            ship::create(
                sim,
                2000.0 + rng.gen_range(-500.0..500.0),
                -2000.0 + rng.gen_range(-500.0..500.0),
                0.0 + rng.gen_range(-10.0..10.0),
                700.0 + rng.gen_range(-300.0..300.0),
                std::f64::consts::PI,
                target(1),
            );
        }
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_tutorial_victory(sim)
    }

    fn solution(&self) -> String {
        include_str!("../../ai/gunnery.rhai").to_string()
    }
}

struct AsteroidStressScenario {}

impl Scenario for AsteroidStressScenario {
    fn name(&self) -> String {
        "asteroid-stress".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        let mut rng = new_rng(seed);
        add_walls(sim);
        ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, fighter(0));

        let bound = (WORLD_SIZE / 2.0) * 0.9;
        for _ in 0..1000 {
            ship::create(
                sim,
                rng.gen_range(-bound..bound),
                rng.gen_range(-bound..bound),
                rng.gen_range(-30.0..30.0),
                rng.gen_range(-30.0..30.0),
                rng.gen_range(0.0..(2.0 * std::f64::consts::PI)),
                asteroid(rng.gen_range(0..30)),
            );
        }
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_tutorial_victory(sim)
    }
}

struct BulletStressScenario {}

impl Scenario for BulletStressScenario {
    fn name(&self) -> String {
        "bullet-stress".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        let mut rng = new_rng(seed);
        add_walls(sim);
        ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, fighter(0));

        let bound = (WORLD_SIZE / 2.0) * 0.9;
        for _ in 0..1000 {
            let s = 1000.0;
            bullet::create(
                sim,
                rng.gen_range(-bound..bound),
                rng.gen_range(-bound..bound),
                rng.gen_range(-s..s),
                rng.gen_range(-s..s),
                BulletData {
                    damage: 10.0,
                    team: 0,
                },
            );
        }
    }

    fn status(&self, sim: &Simulation) -> Status {
        if sim.bullets.is_empty() {
            Status::Victory { team: 0 }
        } else {
            Status::Running
        }
    }
}

struct WelcomeScenario {
    rng: Option<SeededRng>,
}

impl WelcomeScenario {
    fn new() -> Self {
        Self {
            rng: None as Option<SeededRng>,
        }
    }
}

impl Scenario for WelcomeScenario {
    fn name(&self) -> String {
        "welcome".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        self.rng = Some(new_rng(seed));
        add_walls(sim);
        sim.upload_code(0, include_str!("../../ai/welcome.rhai"));
        ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, fighter(0));
    }

    fn tick(&mut self, sim: &mut Simulation) {
        let rng = self.rng.as_mut().unwrap();
        let asteroid_variants = [1, 6, 14];
        while sim.ships.len() < 20 {
            let p = Rotation2::new(rng.gen_range(0.0..std::f64::consts::TAU))
                .transform_point(&point![rng.gen_range(500.0..2000.0), 0.0]);
            ship::create(
                sim,
                p.x,
                p.y,
                rng.gen_range(-30.0..30.0),
                rng.gen_range(-30.0..30.0),
                rng.gen_range(0.0..(2.0 * std::f64::consts::PI)),
                asteroid(*asteroid_variants.choose(rng).unwrap()),
            );
        }
    }

    fn initial_code(&self) -> String {
        "\
// Welcome to Oort.
// Select a scenario from the list in the top-right of the page.
// If you're new, start with \"tutorial01\"."
            .to_string()
    }
}

struct Tutorial01 {}

impl Scenario for Tutorial01 {
    fn name(&self) -> String {
        "tutorial01".into()
    }

    fn init(&mut self, sim: &mut Simulation, _seed: u32) {
        add_walls(sim);
        ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, fighter(0));
        ship::create(sim, 100.0, 0.0, 0.0, 0.0, 0.1, asteroid(1));
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_tutorial_victory(sim)
    }

    fn initial_code(&self) -> String {
        include_str!("../../ai/tutorial/tutorial01.initial.rhai").to_string()
    }

    fn solution(&self) -> String {
        include_str!("../../ai/tutorial/tutorial01.solution.rhai").to_string()
    }

    fn next_scenario(&self) -> Option<String> {
        Some("tutorial02".to_string())
    }
}

struct Tutorial02 {
    hit_target: bool,
}

impl Tutorial02 {
    fn new() -> Self {
        Self { hit_target: false }
    }
}

impl Scenario for Tutorial02 {
    fn name(&self) -> String {
        "tutorial02".into()
    }

    fn init(&mut self, sim: &mut Simulation, _seed: u32) {
        add_walls(sim);
        ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, fighter(0));
        if let Some(&handle) = sim.ships.iter().next() {
            if let Some(c) = sim.ship_controllers.get_mut(&handle) {
                c.write_target(vector![200.0, 0.0]);
            }
        }
    }

    fn tick(&mut self, sim: &mut Simulation) {
        if let Some(&handle) = sim.ships.iter().next() {
            let ship = sim.ship(handle);
            if (ship.position().vector - Translation2::new(200.0, 0.0).vector).magnitude() < 50.0 {
                self.hit_target = true;
            }
        }
    }

    fn lines(&self) -> Vec<Line> {
        let mut lines = vec![];
        let center: Point2<f64> = point![200.0, 0.0];
        let n = 20;
        let r = 50.0;
        let color = if self.hit_target {
            vector![0.0, 1.0, 0.0, 1.0]
        } else {
            vector![1.0, 0.0, 0.0, 1.0]
        };
        for i in 0..n {
            let frac = (i as f64) / (n as f64);
            let angle_a = std::f64::consts::TAU * frac;
            let angle_b = std::f64::consts::TAU * (frac + 1.0 / n as f64);
            lines.push(Line {
                a: center + vector![r * angle_a.cos(), r * angle_a.sin()],
                b: center + vector![r * angle_b.cos(), r * angle_b.sin()],
                color,
            });
        }
        lines
    }

    fn status(&self, _: &Simulation) -> Status {
        if self.hit_target {
            Status::Victory { team: 0 }
        } else {
            Status::Running
        }
    }

    fn initial_code(&self) -> String {
        include_str!("../../ai/tutorial/tutorial02.initial.rhai").to_string()
    }

    fn solution(&self) -> String {
        include_str!("../../ai/tutorial/tutorial02.solution.rhai").to_string()
    }

    fn next_scenario(&self) -> Option<String> {
        Some("tutorial03".to_string())
    }
}

struct Tutorial03 {
    hit_target: bool,
    target: Option<Point2<f64>>,
}

impl Tutorial03 {
    fn new() -> Self {
        Self {
            hit_target: false,
            target: None,
        }
    }
}

impl Scenario for Tutorial03 {
    fn name(&self) -> String {
        "tutorial03".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        let mut rng = new_rng(seed);
        let size = 500.0;
        let range = -size..size;
        self.target = Some(point![rng.gen_range(range.clone()), rng.gen_range(range)]);
        add_walls(sim);
        ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, fighter(0));
        if let Some(&handle) = sim.ships.iter().next() {
            if let Some(c) = sim.ship_controllers.get_mut(&handle) {
                c.write_target(self.target.unwrap().coords);
            }
        }
    }

    fn tick(&mut self, sim: &mut Simulation) {
        if let Some(&handle) = sim.ships.iter().next() {
            let ship = sim.ship(handle);
            if (ship.position().vector - self.target.unwrap().coords).magnitude() < 50.0 {
                self.hit_target = true;
            }
        }
    }

    fn lines(&self) -> Vec<Line> {
        let mut lines = vec![];
        let center: Point2<f64> = self.target.unwrap();
        let n = 20;
        let r = 50.0;
        let color = if self.hit_target {
            vector![0.0, 1.0, 0.0, 1.0]
        } else {
            vector![1.0, 0.0, 0.0, 1.0]
        };
        for i in 0..n {
            let frac = (i as f64) / (n as f64);
            let angle_a = std::f64::consts::TAU * frac;
            let angle_b = std::f64::consts::TAU * (frac + 1.0 / n as f64);
            lines.push(Line {
                a: center + vector![r * angle_a.cos(), r * angle_a.sin()],
                b: center + vector![r * angle_b.cos(), r * angle_b.sin()],
                color,
            });
        }
        lines
    }

    fn status(&self, _: &Simulation) -> Status {
        if self.hit_target {
            Status::Victory { team: 0 }
        } else {
            Status::Running
        }
    }

    fn initial_code(&self) -> String {
        include_str!("../../ai/tutorial/tutorial03.initial.rhai").to_string()
    }

    fn solution(&self) -> String {
        include_str!("../../ai/tutorial/tutorial03.solution.rhai").to_string()
    }

    fn next_scenario(&self) -> Option<String> {
        Some("tutorial04".to_string())
    }
}

struct Tutorial04 {}

impl Tutorial04 {
    fn new() -> Self {
        Self {}
    }
}

impl Scenario for Tutorial04 {
    fn name(&self) -> String {
        "tutorial04".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        add_walls(sim);
        let mut rng = new_rng(seed);
        let size = 500.0;
        let range = -size..size;
        let target = point![rng.gen_range(range.clone()), rng.gen_range(range)];
        ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, fighter(0));
        if let Some(&handle) = sim.ships.iter().next() {
            if let Some(c) = sim.ship_controllers.get_mut(&handle) {
                c.write_target(target.coords);
            }
        }
        ship::create(sim, target.x, target.y, 0.0, 0.0, 0.0, asteroid(1));
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_tutorial_victory(sim)
    }

    fn initial_code(&self) -> String {
        include_str!("../../ai/tutorial/tutorial04.initial.rhai").to_string()
    }

    fn solution(&self) -> String {
        include_str!("../../ai/tutorial/tutorial04.solution.rhai").to_string()
    }

    fn next_scenario(&self) -> Option<String> {
        Some("tutorial05".to_string())
    }
}

struct Tutorial05 {
    ship_handle: Option<ShipHandle>,
    target_handle: Option<ShipHandle>,
}

impl Tutorial05 {
    fn new() -> Self {
        Self {
            ship_handle: None,
            target_handle: None,
        }
    }
}

impl Scenario for Tutorial05 {
    fn name(&self) -> String {
        "tutorial05".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        add_walls(sim);
        self.ship_handle = Some(ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, fighter(0)));

        let mut rng = new_rng(seed);
        let size = 500.0;
        let range = -size..size;
        let target = point![rng.gen_range(range.clone()), rng.gen_range(range)];
        self.target_handle = Some(ship::create(
            sim,
            target.x,
            target.y,
            rng.gen_range(0.0..std::f64::consts::TAU),
            rng.gen_range(-400.0..400.0),
            rng.gen_range(-400.0..400.0),
            fighter(1),
        ));

        if let Some(c) = sim.ship_controllers.get_mut(&self.ship_handle.unwrap()) {
            c.write_target(target.coords);
        }

        sim.upload_code(1, include_str!("../../ai/tutorial/tutorial05.enemy.rhai"));
    }

    fn tick(&mut self, sim: &mut Simulation) {
        if sim.ships.len() < 2 {
            return;
        }
        {
            let target_position = sim.ship(self.target_handle.unwrap()).position();
            if let Some(c) = sim.ship_controllers.get_mut(&self.ship_handle.unwrap()) {
                c.write_target(target_position.vector);
            }
        }
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_tutorial_victory(sim)
    }

    fn initial_code(&self) -> String {
        include_str!("../../ai/tutorial/tutorial05.initial.rhai").to_string()
    }

    fn solution(&self) -> String {
        include_str!("../../ai/tutorial/tutorial05.solution.rhai").to_string()
    }

    fn next_scenario(&self) -> Option<String> {
        Some("tutorial06".to_string())
    }
}

struct Tutorial06 {}

impl Tutorial06 {
    fn new() -> Self {
        Self {}
    }
}

impl Scenario for Tutorial06 {
    fn name(&self) -> String {
        "tutorial06".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        add_walls(sim);
        ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, fighter(0));

        let mut rng = new_rng(seed);
        let size = 500.0;
        let range = -size..size;
        for _ in 0..3 {
            let target = point![rng.gen_range(range.clone()), rng.gen_range(range.clone())];
            ship::create(
                sim,
                target.x,
                target.y,
                rng.gen_range(0.0..std::f64::consts::TAU),
                rng.gen_range(-400.0..400.0),
                rng.gen_range(-400.0..400.0),
                fighter(1),
            );
        }

        sim.upload_code(1, include_str!("../../ai/tutorial/tutorial06.enemy.rhai"));
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_tutorial_victory(sim)
    }

    fn initial_code(&self) -> String {
        include_str!("../../ai/tutorial/tutorial06.initial.rhai").to_string()
    }

    fn solution(&self) -> String {
        include_str!("../../ai/tutorial/tutorial06.solution.rhai").to_string()
    }

    fn next_scenario(&self) -> Option<String> {
        Some("tutorial07".to_string())
    }
}

struct Tutorial07 {}

impl Tutorial07 {
    fn new() -> Self {
        Self {}
    }
}

impl Scenario for Tutorial07 {
    fn name(&self) -> String {
        "tutorial07".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        add_walls(sim);

        sim.upload_code(1, include_str!("../../ai/tutorial/tutorial07.enemy.rhai"));

        let mut rng = new_rng(seed);
        for team in 0..2 {
            for _ in 0..10 {
                let size = 500.0;
                let range = -size..size;
                let center = point![(team as f64 - 0.5) * 1000.0, 0.0];
                let offset = point![rng.gen_range(range.clone()), rng.gen_range(range.clone())];
                let heading = if team == 0 { 0.0 } else { std::f64::consts::PI };
                ship::create(
                    sim,
                    center.x + offset.x,
                    center.y + offset.y,
                    0.0,
                    0.0,
                    heading,
                    fighter(team),
                );
            }
        }
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_tutorial_victory(sim)
    }

    fn initial_code(&self) -> String {
        include_str!("../../ai/tutorial/tutorial07.initial.rhai").to_string()
    }

    fn solution(&self) -> String {
        include_str!("../../ai/tutorial/tutorial07.solution.rhai").to_string()
    }

    fn next_scenario(&self) -> Option<String> {
        Some("tutorial08".to_string())
    }
}

struct Tutorial08 {}

impl Tutorial08 {
    fn new() -> Self {
        Self {}
    }
}

impl Scenario for Tutorial08 {
    fn name(&self) -> String {
        "tutorial08".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        add_walls(sim);

        sim.upload_code(1, include_str!("../../ai/tutorial/tutorial08.enemy.rhai"));

        let mut rng = new_rng(seed);
        {
            for _ in 0..3 {
                let position = Rotation2::new(rng.gen_range(0.0..std::f64::consts::TAU))
                    .transform_point(&point![rng.gen_range(100.0..500.0), 0.0]);
                ship::create(
                    sim,
                    position.x,
                    position.y,
                    0.0,
                    0.0,
                    rng.gen_range(0.0..std::f64::consts::TAU),
                    fighter(0),
                );
            }
        }
        {
            for _ in 0..3 {
                let position = Rotation2::new(rng.gen_range(0.0..std::f64::consts::TAU))
                    .transform_point(&point![rng.gen_range(3500.0..4500.0), 0.0]);
                ship::create(
                    sim,
                    position.x,
                    position.y,
                    0.0,
                    0.0,
                    rng.gen_range(0.0..std::f64::consts::TAU),
                    fighter(1),
                );
            }
        }
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_tutorial_victory(sim)
    }

    fn initial_code(&self) -> String {
        include_str!("../../ai/tutorial/tutorial08.initial.rhai").to_string()
    }

    fn solution(&self) -> String {
        include_str!("../../ai/tutorial/tutorial08.solution.rhai").to_string()
    }

    fn next_scenario(&self) -> Option<String> {
        Some("tutorial09".to_string())
    }
}

struct Tutorial09 {}

impl Tutorial09 {
    fn new() -> Self {
        Self {}
    }
}

impl Scenario for Tutorial09 {
    fn name(&self) -> String {
        "tutorial09".into()
    }

    fn init(&mut self, sim: &mut Simulation, seed: u32) {
        add_walls(sim);

        sim.upload_code(1, include_str!("../../ai/tutorial/tutorial09.enemy.rhai"));

        let mut shipdata = fighter(0);
        shipdata.weapons.clear();
        ship::create(sim, 0.0, 0.0, 0.0, 0.0, 0.0, shipdata);

        let mut rng = new_rng(seed);
        for _ in 0..3 {
            let p = Rotation2::new(rng.gen_range(0.0..std::f64::consts::TAU))
                .transform_vector(&vector![rng.gen_range(1000.0..1500.0), 0.0]);
            let v = Rotation2::new(rng.gen_range(0.0..std::f64::consts::TAU))
                .transform_vector(&vector![rng.gen_range(0.0..300.0), 0.0]);
            ship::create(sim, p.x, p.y, v.x, v.y, std::f64::consts::PI, fighter(1));
        }
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_tutorial_victory(sim)
    }

    fn initial_code(&self) -> String {
        include_str!("../../ai/tutorial/tutorial09.initial.rhai").to_string()
    }

    fn solution(&self) -> String {
        include_str!("../../ai/tutorial/tutorial09.solution.rhai").to_string()
    }
}

struct Duel {}

impl Duel {
    fn new() -> Self {
        Self {}
    }
}

impl Scenario for Duel {
    fn name(&self) -> String {
        "duel".into()
    }

    fn init(&mut self, sim: &mut Simulation, _seed: u32) {
        add_walls(sim);
        sim.upload_code(1, include_str!("../../ai/duel.reference.rhai"));
        ship::create(sim, -1000.0, 0.0, 0.0, 0.0, 0.0, fighter(0));
        ship::create(sim, 1000.0, 0.0, 0.0, 0.0, std::f64::consts::PI, fighter(1));
    }

    fn status(&self, sim: &Simulation) -> Status {
        check_victory(sim)
    }

    fn initial_code(&self) -> String {
        include_str!("../../ai/duel.initial.rhai").to_string()
    }

    fn solution(&self) -> String {
        include_str!("../../ai/duel.reference.rhai").to_string()
    }
}