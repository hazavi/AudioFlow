export type Theme = "dark" | "light" | "system";
export type Page =
  | "Dashboard"
  | "Monitors"
  | "Applications"
  | "Routing Rules"
  | "Activity"
  | "Settings";
export type RoutingMode = "auto" | "fixed" | "default" | "ignore";
export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface Monitor {
  id: string;
  name: string;
  deviceName: string;
  bounds: Bounds;
  primary: boolean;
  refreshRate: number | null;
  scale: number | null;
}
export interface Device {
  id: string;
  name: string;
  isDefault: boolean;
}
export interface Session {
  pid: number;
  endpointId: string;
  active: boolean;
  peak: number | null;
}
export interface Rule {
  id: string;
  executable: string;
  mode: RoutingMode;
  endpointId: string | null;
  enabled: boolean;
  priority: number;
  createdAt: string;
  updatedAt: string;
}
export interface Settings {
  routingEnabled: boolean;
  launchOnStartup: boolean;
  startMinimized: boolean;
  minimizeToTray: boolean;
  notifications: boolean;
  switchDelayMs: number;
  routeNewApps: boolean;
  restoreOnExit: boolean;
  respectManual: boolean;
  fallbackEndpoint: string | null;
  reconnectBehavior: "wait" | "fallback";
  debugLogging: boolean;
  theme: Theme;
  onboardingCompleted: boolean;
  automaticUpdates: boolean;
  updateNotifications: boolean;
}
export interface Config {
  schemaVersion: number;
  settings: Settings;
  assignments: Record<string, string>;
  rules: Rule[];
}
export interface Application {
  iconData: string | null;
  pid: number;
  name: string;
  executable: string;
  monitorId: string | null;
  endpointIds: string[];
  active: boolean;
  peak: number | null;
  mode: RoutingMode;
  status: string;
  lastSwitch: string | null;
  lastError: string | null;
  windowCount: number;
}
export interface Activity {
  id: string;
  timestamp: string;
  eventType: string;
  application: string | null;
  previousDevice: string | null;
  newDevice: string | null;
  outcome: string;
  message: string;
}
export interface Snapshot {
  monitors: Monitor[];
  devices: Device[];
  sessions: Session[];
  applications: Application[];
  config: Config;
  activity: Activity[];
  routingSupported: boolean;
  compatibility: string;
  successfulChanges: number;
  lastScan: string | null;
  errors: string[];
  version: string;
  repository: string | null;
  updaterConfigured: boolean;
}
