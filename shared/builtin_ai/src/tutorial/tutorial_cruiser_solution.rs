// Tutorial: Cruiser (solution)
// Destroy the enemy ships with your Cruiser.
use oort_api::prelude::*;

pub struct Ship {}

impl Ship {
    pub fn new() -> Ship {
        set_radar_heading(heading());
        Ship {}
    }

    pub fn tick(&mut self) {
        if class() == Class::Missile {
            if let Some(contact) = scan() {
                seek(contact.position, contact.velocity);

                let dp = contact.position - position();
                let dv = contact.velocity - velocity();
                if dp.length().min((dp + dv * TICK_LENGTH).length()) < 25.0 {
                    explode();
                }

                set_radar_heading((contact.position - position()).angle());
                set_radar_width((10.0 * TAU / dp.length()).clamp(TAU / 30.0, TAU));
            } else {
                accelerate(vec2(100.0, 0.0).rotate(heading()));
                set_radar_width(TAU / 32.0);
                set_radar_heading(radar_heading() + radar_width());
            }
        } else {
            set_radar_width(TAU / 32.0);
            if let Some(contact) = scan() {
                fire(1);
                fire(2);

                aim(0, lead_target(contact.position, contact.velocity, 2000.0));
                fire(0);

                let dp = contact.position - position();
                turn_to(dp.angle());
                set_radar_heading(dp.angle());
            } else {
                set_radar_heading(radar_heading() + TAU / 32.0);
            }
            turn(1.0);
        }
    }
}

pub fn seek(p: Vec2, v: Vec2) {
    let dp = p - position();
    let dv = v - velocity();
    let closing_speed = -(dp.y * dv.y - dp.x * dv.x).abs() / dp.length();
    let los = dp.angle();
    let los_rate = (dp.y * dv.x - dp.x * dv.y) / (dp.length() * dp.length());

    const N: f64 = 4.0;
    let a = vec2(100.0, N * closing_speed * los_rate).rotate(los);
    let a = vec2(400.0, 0.0).rotate(a.angle());
    accelerate(a);
    turn_to(a.angle());
}

fn turn_to(target_heading: f64) {
    let heading_error = angle_diff(heading(), target_heading);
    turn(10.0 * heading_error);
}

fn lead_target(target_position: Vec2, target_velocity: Vec2, bullet_speed: f64) -> f64 {
    let dp = target_position - position();
    let dv = target_velocity - velocity();
    let predicted_dp = dp + dv * dp.length() / bullet_speed;
    predicted_dp.angle()
}
