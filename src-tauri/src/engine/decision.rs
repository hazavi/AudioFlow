use crate::models::{AppWindow, Bounds, Config, Device, Monitor, RoutingMode, Rule};

pub fn intersection(a: Bounds, b: Bounds) -> i64 {
    let w = (i64::from(a.x) + i64::from(a.width)).min(i64::from(b.x) + i64::from(b.width))
        - i64::from(a.x.max(b.x));
    let h = (i64::from(a.y) + i64::from(a.height)).min(i64::from(b.y) + i64::from(b.height))
        - i64::from(a.y.max(b.y));
    w.max(0) * h.max(0)
}
pub fn select_monitor<'a>(
    bounds: Bounds,
    monitors: &'a [Monitor],
    previous: Option<&str>,
) -> Option<&'a Monitor> {
    let best = monitors
        .iter()
        .max_by_key(|m| (intersection(bounds, m.bounds), m.primary))?;
    let largest = intersection(bounds, best.bounds);
    if largest == 0 {
        return None;
    }
    if let Some(old) = monitors.iter().find(|m| Some(m.id.as_str()) == previous) {
        // A ten-percent area margin avoids chatter around a display boundary.
        let margin = i64::from(bounds.width.max(0)) * i64::from(bounds.height.max(0)) / 10;
        if intersection(bounds, old.bounds) > 0
            && largest - intersection(bounds, old.bounds) <= margin
        {
            return Some(old);
        }
    }
    Some(best)
}
pub fn select_window<'a>(
    windows: &'a [AppWindow],
    previous: Option<isize>,
) -> Option<&'a AppWindow> {
    windows
        .iter()
        .find(|w| w.focused)
        .or_else(|| windows.iter().find(|w| Some(w.handle) == previous))
        .or_else(|| {
            windows.iter().max_by_key(|w| {
                (
                    i64::from(w.bounds.width) * i64::from(w.bounds.height),
                    -w.handle,
                )
            })
        })
}
pub fn matching_rule<'a>(config: &'a Config, executable: &str) -> Option<&'a Rule> {
    let path = executable.to_lowercase();
    let basename = path.rsplit(['\\', '/']).next().unwrap_or(&path);
    let mut matches: Vec<_> = config
        .rules
        .iter()
        .filter(|r| {
            let pattern = r.executable.to_lowercase();
            r.enabled && (pattern == path || pattern == basename)
        })
        .collect();
    matches.sort_by_key(|r| {
        (
            std::cmp::Reverse(r.priority),
            match r.mode {
                RoutingMode::Fixed => 0,
                RoutingMode::Ignore => 1,
                RoutingMode::Auto => 2,
                RoutingMode::Default => 3,
            },
            &r.id,
        )
    });
    matches.first().copied()
}
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Leave(&'static str),
    Route(Option<String>),
}
pub fn decide(
    config: &Config,
    executable: &str,
    monitor: Option<&str>,
    devices: &[Device],
    has_manual: bool,
) -> Decision {
    if !config.settings.routing_enabled {
        return Decision::Leave("Routing paused");
    }
    let rule = matching_rule(config, executable);
    if rule.is_some_and(|r| r.mode == RoutingMode::Ignore) {
        return Decision::Leave("Ignored by rule");
    }
    if has_manual && config.settings.respect_manual && rule.is_none() {
        return Decision::Leave("Windows manual assignment");
    }
    let mode = rule.map(|r| &r.mode).unwrap_or(&RoutingMode::Auto);
    let endpoint = match mode {
        RoutingMode::Default => return Decision::Route(None),
        RoutingMode::Fixed => rule.and_then(|r| r.endpoint_id.clone()),
        RoutingMode::Auto => monitor.and_then(|id| config.assignments.get(id).cloned()),
        RoutingMode::Ignore => return Decision::Leave("Ignored by rule"),
    };
    let Some(endpoint) = endpoint else {
        return Decision::Leave("No output assigned");
    };
    if endpoint == "default" {
        return Decision::Route(None);
    }
    if devices.iter().any(|d| d.id == endpoint) {
        return Decision::Route(Some(endpoint));
    }
    if config.settings.reconnect_behavior == "fallback" {
        match &config.settings.fallback_endpoint {
            Some(id) if id == "default" => return Decision::Route(None),
            Some(id) if devices.iter().any(|d| &d.id == id) => {
                return Decision::Route(Some(id.clone()))
            }
            _ => {}
        }
    }
    Decision::Leave("Assigned output disconnected; waiting for reconnection")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn monitor(id: &str, x: i32, y: i32) -> Monitor {
        Monitor {
            id: id.into(),
            name: id.into(),
            device_name: id.into(),
            bounds: Bounds {
                x,
                y,
                width: 1920,
                height: 1080,
            },
            primary: x == 0,
            refresh_rate: None,
            scale: None,
        }
    }
    fn devices() -> Vec<Device> {
        vec![Device {
            id: "speaker".into(),
            name: "Speaker".into(),
            is_default: true,
        }]
    }
    #[test]
    fn negative_coordinates_vertical_and_hysteresis() {
        let monitors = vec![
            monitor("left", -1920, 0),
            monitor("main", 0, 0),
            monitor("above", 0, -1080),
        ];
        assert_eq!(
            select_monitor(
                Bounds {
                    x: -1500,
                    y: 20,
                    width: 800,
                    height: 600
                },
                &monitors,
                None
            )
            .unwrap()
            .id,
            "left"
        );
        assert_eq!(
            select_monitor(
                Bounds {
                    x: 100,
                    y: -1000,
                    width: 800,
                    height: 600
                },
                &monitors,
                None
            )
            .unwrap()
            .id,
            "above"
        );
        assert_eq!(
            select_monitor(
                Bounds {
                    x: -490,
                    y: 0,
                    width: 1000,
                    height: 600
                },
                &monitors,
                Some("left")
            )
            .unwrap()
            .id,
            "left"
        );
        assert_eq!(
            select_monitor(
                Bounds {
                    x: -300,
                    y: 0,
                    width: 1000,
                    height: 600
                },
                &monitors,
                Some("left")
            )
            .unwrap()
            .id,
            "main"
        );
        assert!(select_monitor(
            Bounds {
                x: 5000,
                y: 0,
                width: 100,
                height: 100
            },
            &monitors,
            None
        )
        .is_none());
    }
    #[test]
    fn manual_and_explicit_rules() {
        let mut c = Config::default();
        c.settings.routing_enabled = true;
        c.assignments.insert("main".into(), "speaker".into());
        assert!(matches!(
            decide(&c, "app.exe", Some("main"), &devices(), true),
            Decision::Leave(_)
        ));
        c.rules.push(Rule {
            id: "a".into(),
            executable: "app.exe".into(),
            mode: RoutingMode::Fixed,
            endpoint_id: Some("speaker".into()),
            enabled: true,
            priority: 10,
            created_at: String::new(),
            updated_at: String::new(),
        });
        assert_eq!(
            decide(&c, "C:\\Apps\\APP.EXE", Some("missing"), &devices(), true),
            Decision::Route(Some("speaker".into()))
        );
        c.rules[0].mode = RoutingMode::Ignore;
        assert_eq!(
            decide(&c, "app.exe", Some("main"), &devices(), false),
            Decision::Leave("Ignored by rule")
        );
        c.settings.routing_enabled = false;
        assert_eq!(
            decide(&c, "app.exe", Some("main"), &devices(), false),
            Decision::Leave("Routing paused")
        );
    }
    #[test]
    fn unplugged_endpoint_requires_explicit_fallback() {
        let mut c = Config::default();
        c.settings.routing_enabled = true;
        c.assignments.insert("main".into(), "unplugged".into());
        assert!(matches!(
            decide(&c, "app.exe", Some("main"), &devices(), false),
            Decision::Leave(_)
        ));
        c.settings.reconnect_behavior = "fallback".into();
        c.settings.fallback_endpoint = Some("speaker".into());
        assert_eq!(
            decide(&c, "app.exe", Some("main"), &devices(), false),
            Decision::Route(Some("speaker".into()))
        );
        assert!(matches!(
            decide(&c, "app.exe", None, &devices(), false),
            Decision::Leave(_)
        ));
    }
    #[test]
    fn focus_then_last_window_then_largest() {
        let mut ws = vec![
            AppWindow {
                handle: 1,
                pid: 2,
                bounds: Bounds {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100,
                },
                focused: false,
            },
            AppWindow {
                handle: 2,
                pid: 2,
                bounds: Bounds {
                    x: 0,
                    y: 0,
                    width: 200,
                    height: 200,
                },
                focused: false,
            },
        ];
        assert_eq!(select_window(&ws, None).unwrap().handle, 2);
        assert_eq!(select_window(&ws, Some(1)).unwrap().handle, 1);
        ws[1].focused = true;
        assert_eq!(select_window(&ws, Some(1)).unwrap().handle, 2);
    }
}
