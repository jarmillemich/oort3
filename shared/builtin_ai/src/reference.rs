use oort_api::prelude::*;

// This enum stores a different struct for each ship class.
pub enum Ship {
    Fighter(Fighter),
    Frigate(Frigate),
    Cruiser(Cruiser),
    Missile(Missile), // Also used for torpedos.
}

impl Ship {
    pub fn new() -> Ship {
        match class() {
            Class::Fighter => Ship::Fighter(Fighter::new()),
            Class::Frigate => Ship::Frigate(Frigate::new()),
            Class::Cruiser => Ship::Cruiser(Cruiser::new()),
            Class::Missile => Ship::Missile(Missile::new()),
            Class::Torpedo => Ship::Missile(Missile::new()),
            _ => unreachable!(),
        }
    }

    pub fn tick(&mut self) {
        match self {
            Ship::Fighter(fighter) => fighter.tick(),
            Ship::Frigate(frigate) => frigate.tick(),
            Ship::Cruiser(cruiser) => cruiser.tick(),
            Ship::Missile(missile) => missile.tick(),
        }
    }
}

// Fighters
pub struct Fighter {
    pub move_target: Vec2,
}

impl Fighter {
    pub fn new() -> Self {
        Self {
            move_target: vec2(0.0, 0.0),
        }
    }

    pub fn tick(&mut self) {
        if let Some(contact) = scan().filter(|c| {
            [
                Class::Fighter,
                Class::Frigate,
                Class::Cruiser,
                Class::Torpedo,
                Class::Asteroid,
            ]
            .contains(&c.class)
        }) {
            let dp = contact.position - position();

            // Point the radar at the target and focus the beam.
            set_radar_heading(dp.angle());
            set_radar_width(radar_width() * 0.5);

            // Fly towards the target.
            seek(contact.position, vec2(0.0, 0.0), true);

            // Guns
            if let Some(angle) = lead_target(contact.position, contact.velocity, 1e3, 10.0) {
                // Random jitter makes it more likely to hit accelerating targets.
                let angle = angle + rand(-1.0, 1.0) * TAU / 120.0;
                turn_to(angle);
                if angle_diff(angle, heading()).abs() < TAU / 60.0 {
                    fire(0);
                }
            }

            // Missiles
            if reload_ticks(1) == 0 {
                // The missile will fly towards this position and acquire the target with radar
                // when close enough.
                send(make_orders(contact.position, contact.velocity));
                fire(1);
            }
        } else {
            // Scan the radar around in a circle.
            set_radar_heading(radar_heading() + radar_width());
            set_radar_width(TAU / 120.0);
            seek(self.move_target, vec2(0.0, 0.0), true);
        }
    }
}

// Frigates
pub struct Frigate {
    pub move_target: Vec2,
    pub radar_state: FrigateRadarState,
    pub main_gun_radar: RadarRegs,
    pub point_defense_radar: RadarRegs,
}

// The ship only has one radar, but we need to track different targets for the main gun and
// missiles versus point defense. We switch between these modes each tick and use this enum to
// track which mode we're in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FrigateRadarState {
    MainGun,
    PointDefense,
}

impl Frigate {
    pub fn new() -> Self {
        Self {
            move_target: vec2(0.0, 0.0),
            radar_state: FrigateRadarState::MainGun,
            main_gun_radar: RadarRegs::new(),
            point_defense_radar: RadarRegs::new(),
        }
    }

    pub fn tick(&mut self) {
        if self.radar_state == FrigateRadarState::MainGun {
            if let Some(contact) = scan().filter(|c| {
                [
                    Class::Fighter,
                    Class::Frigate,
                    Class::Cruiser,
                    Class::Asteroid,
                ]
                .contains(&c.class)
            }) {
                self.move_target = contact.position;
                let dp = contact.position - position();
                set_radar_heading(dp.angle());
                set_radar_width(radar_width() * 0.5);

                // Main gun
                if let Some(angle) = lead_target(contact.position, contact.velocity, 4e3, 60.0) {
                    turn_to(angle);
                    if angle_diff(angle, heading()).abs() < TAU / 360.0 {
                        fire(0);
                    }
                }

                // Missiles
                if reload_ticks(3) == 0 {
                    send(make_orders(contact.position, contact.velocity));
                    fire(3);
                }
            } else {
                self.move_target = vec2(0.0, 0.0);
                set_radar_heading(radar_heading() + radar_width());
                set_radar_width(TAU / 120.0);
            }

            // Switch to the next radar mode.
            self.main_gun_radar.save();
            self.point_defense_radar.restore();
            self.radar_state = FrigateRadarState::PointDefense;
        } else if self.radar_state == FrigateRadarState::PointDefense {
            // Point defense cares about very close targets and needs to cover 360 degrees as
            // frequently as possible.
            set_radar_width(TAU / 4.0);
            set_radar_max_distance(1e3);

            if let Some(contact) = scan().filter(|c| {
                [
                    Class::Fighter,
                    Class::Missile,
                    Class::Torpedo,
                    Class::Asteroid,
                ]
                .contains(&c.class)
            }) {
                for idx in [1, 2] {
                    if let Some(angle) = lead_target(contact.position, contact.velocity, 1e3, 10.0)
                    {
                        aim(idx, angle + rand(-1.0, 1.0) * TAU / 120.0);
                        fire(idx);
                    }
                }
                set_radar_heading((contact.position - position()).angle());
            } else {
                set_radar_heading(radar_heading() + radar_width());
            }

            self.point_defense_radar.save();
            self.main_gun_radar.restore();
            self.radar_state = FrigateRadarState::MainGun;
        }

        seek(self.move_target, vec2(0.0, 0.0), true);
    }
}

// Cruisers
pub struct Cruiser {
    pub move_target: Vec2,
    pub radar_state: CruiserRadarState,
    pub torpedo_radar: RadarRegs,
    pub missile_radar: RadarRegs,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CruiserRadarState {
    Torpedo,
    Missile,
}

impl Cruiser {
    pub fn new() -> Self {
        Self {
            move_target: vec2(0.0, 0.0),
            radar_state: CruiserRadarState::Torpedo,
            torpedo_radar: RadarRegs::new(),
            missile_radar: RadarRegs::new(),
        }
    }

    pub fn tick(&mut self) {
        seek(self.move_target, vec2(0.0, 0.0), true);

        if self.radar_state == CruiserRadarState::Torpedo {
            if let Some(contact) = scan()
                .filter(|c| [Class::Frigate, Class::Cruiser, Class::Asteroid].contains(&c.class))
            {
                let dp = contact.position - position();
                set_radar_heading(dp.angle());
                set_radar_width(radar_width() * 0.5);

                if reload_ticks(3) == 0 {
                    send(make_orders(contact.position, contact.velocity));
                    fire(3);
                }

                if let Some(angle) = lead_target(contact.position, contact.velocity, 2e3, 120.0) {
                    aim(0, angle);
                    fire(0);
                }
            } else {
                set_radar_heading(radar_heading() + radar_width());
                set_radar_width(TAU / 120.0);
            }

            self.torpedo_radar.save();
            self.missile_radar.restore();
            self.radar_state = CruiserRadarState::Missile;
        } else if self.radar_state == CruiserRadarState::Missile {
            set_radar_width(TAU / 8.0);

            if let Some(contact) = scan().filter(|c| {
                [
                    Class::Fighter,
                    Class::Frigate,
                    Class::Cruiser,
                    Class::Torpedo,
                    Class::Asteroid,
                ]
                .contains(&c.class)
            }) {
                // Only fire one missile at each target.
                let mut fired = false;
                for idx in [1, 2] {
                    if reload_ticks(idx) == 0 {
                        send(make_orders(contact.position, contact.velocity));
                        fire(idx);
                        fired = true;
                        break;
                    }
                }
                if fired {
                    set_radar_heading(radar_heading() + radar_width());
                } else {
                    set_radar_heading((contact.position - position()).angle());
                }
            } else {
                set_radar_heading(radar_heading() + radar_width());
            }

            self.missile_radar.save();
            self.torpedo_radar.restore();
            self.radar_state = CruiserRadarState::Torpedo;
        }
    }
}

// Missiles and Torpedos
pub struct Missile {
    target_position: Vec2,
    target_velocity: Vec2,
}

impl Missile {
    pub fn new() -> Self {
        let (target_position, target_velocity) = parse_orders(receive());
        Self {
            target_position,
            target_velocity,
        }
    }

    pub fn tick(&mut self) {
        self.target_position += self.target_velocity * TICK_LENGTH;

        // Don't let torpedos get distracted by smaller ships.
        let missile_target_classes = [
            Class::Fighter,
            Class::Frigate,
            Class::Cruiser,
            Class::Torpedo,
        ];
        let torpedo_target_classes = [Class::Frigate, Class::Cruiser];
        let target_classes = if class() == Class::Missile {
            missile_target_classes.as_slice()
        } else {
            torpedo_target_classes.as_slice()
        };

        if let Some(contact) = scan().filter(|c| target_classes.contains(&c.class)) {
            let dp = contact.position - position();
            set_radar_heading(dp.angle());
            set_radar_width(radar_width() * 0.5);
            self.target_position = contact.position;
            self.target_velocity = contact.velocity;
        } else {
            // Search near the predicted target area.
            set_radar_heading(
                (self.target_position - position()).angle() + rand(-1.0, 1.0) * TAU / 32.0,
            );
            set_radar_width(TAU / 120.0);
        }

        seek(self.target_position, self.target_velocity, true);
    }
}

// Library functions

/// Flies towards a target which has the given position and velocity.
pub fn seek(p: Vec2, v: Vec2, turn: bool) {
    let dp = p - position();
    let dv = v - velocity();
    let low_fuel = fuel() != 0.0 && fuel() < 500.0;

    // Component of dv perpendicular to dp
    let badv = -(dv - dv.dot(dp) * dp.normalize() / dp.length());
    // Acceleration towards the target
    let forward = if low_fuel { vec2(0.0, 0.0) } else { dp };
    let a = (forward - badv * 10.0).normalize() * max_forward_acceleration();
    accelerate(a);

    if turn {
        turn_to(a.angle());
    }
}

/// Turns towards the given heading.
fn turn_to(target_heading: f64) {
    let heading_error = angle_diff(heading(), target_heading);
    turn(10.0 * heading_error);
}

/// Returns the angle at which to shoot to hit the given target.
fn lead_target(
    target_position: Vec2,
    target_velocity: Vec2,
    bullet_speed: f64,
    ttl: f64,
) -> Option<f64> {
    let dp = target_position - position();
    let dv = target_velocity - velocity();
    let predicted_dp = dp + dv * dp.length() / bullet_speed;
    if predicted_dp.length() / bullet_speed < ttl {
        Some(predicted_dp.angle())
    } else {
        None
    }
}

/// Constructs a radio message from two vectors.
fn make_orders(p: Vec2, v: Vec2) -> Message {
    [p.x, p.y, v.x, v.y]
}

/// Reverse of make_orders.
fn parse_orders(msg: Option<Message>) -> (Vec2, Vec2) {
    if let Some(msg) = msg {
        (vec2(msg[0], msg[1]), vec2(msg[2], msg[3]))
    } else {
        (vec2(0.0, 0.0), vec2(0.0, 0.0))
    }
}

/// Save and restore radar registers in order to use a single radar for multiple functions.
pub struct RadarRegs {
    heading: f64,
    width: f64,
    min_distance: f64,
    max_distance: f64,
}

impl RadarRegs {
    fn new() -> Self {
        Self {
            heading: 0.0,
            width: TAU / 120.0,
            min_distance: 0.0,
            max_distance: 1e9,
        }
    }

    fn save(&mut self) {
        self.heading = radar_heading();
        self.width = radar_width();
        self.min_distance = radar_min_distance();
        self.max_distance = radar_max_distance();
    }

    fn restore(&self) {
        set_radar_heading(self.heading);
        set_radar_width(self.width);
        set_radar_min_distance(self.min_distance);
        set_radar_max_distance(self.max_distance);
    }
}
