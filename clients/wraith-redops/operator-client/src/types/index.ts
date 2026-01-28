// Type definitions matching Rust JSON types from lib.rs

export interface Implant {
  id: string;
  campaign_id: string;
  hostname: string;
  internal_ip: string;
  external_ip: string;
  os_type: string;
  os_version: string;
  architecture: string;
  username: string;
  domain: string;
  privileges: string;
  implant_version: string;
  checkin_interval: number;
  jitter_percent: number;
  status: string;
}

export interface Campaign {
  id: string;
  name: string;
  description: string;
  status: string;
  implant_count: number;
  active_implant_count: number;
}

export interface Listener {
  id: string;
  name: string;
  type_: string;
  bind_address: string;
  port: number;
  status: string;
}

export interface Command {
  id: string;
  implant_id: string;
  command_type: string;
  status: string;
  payload_preview: string;
}

export interface CommandResult {
  id: string;
  output: string;
  exit_code: number;
  error_message: string;
}

export interface Credential {
  id: string;
  implant_id: string;
  source: string;
  credential_type: string;
  domain: string;
  username: string;
}

export interface Artifact {
  id: string;
  filename: string;
  size: number;
}

export interface PersistenceItem {
  id: string;
  implant_id: string;
  method: string;
  details: string;
}

export interface ChainStep {
  id: string;
  step_order: number;
  technique_id: string;
  command_type: string;
  payload: string;
  description: string;
}

export interface ChainStepInput {
  step_order: number;
  technique_id: string;
  command_type: string;
  payload: string;
  description: string;
}

export interface AttackChain {
  id: string;
  name: string;
  description: string;
  steps: ChainStep[];
}

export interface Playbook {
  id: string;
  name: string;
  description: string;
  content: string;
}

export interface StreamEvent {
  id: string;
  type: string;
  implant_id: string;
  data: Record<string, string>;
}
