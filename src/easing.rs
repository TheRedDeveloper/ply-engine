use core::f32::consts::PI;

#[inline]
pub fn ease_in_quad(t: f32) -> f32 {
    t * t
}

#[inline]
pub fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

#[inline]
pub fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

#[inline]
pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

#[inline]
pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

#[inline]
pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

#[inline]
pub fn ease_in_quart(t: f32) -> f32 {
    t * t * t * t
}

#[inline]
pub fn ease_out_quart(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(4)
}

#[inline]
pub fn ease_in_out_quart(t: f32) -> f32 {
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
    }
}

#[inline]
pub fn ease_in_sine(t: f32) -> f32 {
    1.0 - ((t * PI) / 2.0).cos()
}

#[inline]
pub fn ease_out_sine(t: f32) -> f32 {
    ((t * PI) / 2.0).sin()
}

#[inline]
pub fn ease_in_out_sine(t: f32) -> f32 {
    -((PI * t).cos() - 1.0) / 2.0
}

#[inline]
pub fn ease_in_expo(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        2.0_f32.powf(10.0 * t - 10.0)
    }
}

#[inline]
pub fn ease_out_expo(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        1.0 - 2.0_f32.powf(-10.0 * t)
    }
}

#[inline]
pub fn ease_in_out_expo(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else if t < 0.5 {
        2.0_f32.powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
    }
}

#[inline]
pub fn ease_in_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    c3 * t * t * t - c1 * t * t
}

#[inline]
pub fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

#[inline]
pub fn ease_in_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c2 = c1 * 1.525;

    if t < 0.5 {
        ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
    } else {
        ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (2.0 * t - 2.0) + c2) + 2.0)
            / 2.0
    }
}

#[inline]
pub fn ease_in_elastic(t: f32) -> f32 {
    let c4 = (2.0 * PI) / 3.0;

    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        -(2.0_f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c4).sin()
    }
}

#[inline]
pub fn ease_out_elastic(t: f32) -> f32 {
    let c4 = (2.0 * PI) / 3.0;

    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
    }
}

#[inline]
pub fn ease_in_out_elastic(t: f32) -> f32 {
    let c5 = (2.0 * PI) / 4.5;

    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else if t < 0.5 {
        -(2.0_f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0
    } else {
        (2.0_f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0 + 1.0
    }
}

#[inline]
fn ease_out_bounce_internal(t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;

    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let x = t - 1.5 / d1;
        n1 * x * x + 0.75
    } else if t < 2.5 / d1 {
        let x = t - 2.25 / d1;
        n1 * x * x + 0.9375
    } else {
        let x = t - 2.625 / d1;
        n1 * x * x + 0.984375
    }
}

#[inline]
pub fn ease_out_bounce(t: f32) -> f32 {
    ease_out_bounce_internal(t)
}

#[inline]
pub fn ease_in_bounce(t: f32) -> f32 {
    1.0 - ease_out_bounce_internal(1.0 - t)
}

#[inline]
pub fn ease_in_out_bounce(t: f32) -> f32 {
    if t < 0.5 {
        (1.0 - ease_out_bounce_internal(1.0 - 2.0 * t)) / 2.0
    } else {
        (1.0 + ease_out_bounce_internal(2.0 * t - 1.0)) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type EaseFn = fn(f32) -> f32;

    fn assert_close(a: f32, b: f32) {
        assert!(
            (a - b).abs() <= 0.0001,
            "expected {} ~= {} (delta {})",
            a,
            b,
            (a - b).abs()
        );
    }

    #[test]
    fn test_all_easing_endpoints() {
        let all: [EaseFn; 24] = [
            ease_in_quad,
            ease_out_quad,
            ease_in_out_quad,
            ease_in_cubic,
            ease_out_cubic,
            ease_in_out_cubic,
            ease_in_quart,
            ease_out_quart,
            ease_in_out_quart,
            ease_in_sine,
            ease_out_sine,
            ease_in_out_sine,
            ease_in_expo,
            ease_out_expo,
            ease_in_out_expo,
            ease_in_elastic,
            ease_out_elastic,
            ease_in_out_elastic,
            ease_in_bounce,
            ease_out_bounce,
            ease_in_out_bounce,
            ease_in_back,
            ease_out_back,
            ease_in_out_back,
        ];

        for ease in all {
            assert_close(ease(0.0), 0.0);
            assert_close(ease(1.0), 1.0);
        }
    }

    #[test]
    fn test_simple_midpoints() {
        assert_close(ease_in_quad(0.5), 0.25);
        assert_close(ease_out_quad(0.5), 0.75);
        assert_close(ease_in_out_quad(0.5), 0.5);

        assert_close(ease_in_cubic(0.5), 0.125);
        assert_close(ease_out_cubic(0.5), 0.875);
        assert_close(ease_in_out_cubic(0.5), 0.5);

        assert_close(ease_in_quart(0.5), 0.0625);
        assert_close(ease_out_quart(0.5), 0.9375);
        assert_close(ease_in_out_quart(0.5), 0.5);

        assert_close(ease_in_sine(0.5), 0.29289323);
        assert_close(ease_out_sine(0.5), 0.70710677);
        assert_close(ease_in_out_sine(0.5), 0.5);

        assert_close(ease_in_expo(0.5), 0.03125);
        assert_close(ease_out_expo(0.5), 0.96875);
        assert_close(ease_in_out_expo(0.5), 0.5);
    }

    #[test]
    fn test_non_overshoot_easing_stays_in_unit_range() {
        let non_overshoot: [EaseFn; 18] = [
            ease_in_quad,
            ease_out_quad,
            ease_in_out_quad,
            ease_in_cubic,
            ease_out_cubic,
            ease_in_out_cubic,
            ease_in_quart,
            ease_out_quart,
            ease_in_out_quart,
            ease_in_sine,
            ease_out_sine,
            ease_in_out_sine,
            ease_in_expo,
            ease_out_expo,
            ease_in_out_expo,
            ease_in_bounce,
            ease_out_bounce,
            ease_in_out_bounce,
        ];

        for ease in non_overshoot {
            for i in 0..=1000 {
                let t = i as f32 / 1000.0;
                let y = ease(t);
                assert!(
                    (0.0..=1.0).contains(&y),
                    "value {} out of range for t={}",
                    y,
                    t
                );
            }
        }
    }

    #[test]
    fn test_back_and_elastic_overshoot() {
        let overshooting: [EaseFn; 6] = [
            ease_in_back,
            ease_out_back,
            ease_in_out_back,
            ease_in_elastic,
            ease_out_elastic,
            ease_in_out_elastic,
        ];

        for ease in overshooting {
            let mut found = false;
            for i in 0..=1000 {
                let t = i as f32 / 1000.0;
                let y = ease(t);
                if !(0.0..=1.0).contains(&y) {
                    found = true;
                    break;
                }
            }
            assert!(found, "expected overshoot for easing function");
        }

        assert!(ease_in_back(0.5) < 0.0);
        assert!(ease_out_back(0.5) > 1.0);
    }
}