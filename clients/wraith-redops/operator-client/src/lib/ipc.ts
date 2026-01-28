import { invoke } from '@tauri-apps/api/core';
import type {
  Implant,
  Campaign,
  Listener,
  Command,
  CommandResult,
  Credential,
  Artifact,
  PersistenceItem,
  AttackChain,
  ChainStepInput,
  Playbook,
} from '../types';

// Helper to invoke + parse JSON string responses
async function invokeJson<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const json = await invoke<string>(cmd, args);
  return JSON.parse(json) as T;
}

// --- Connection ---

export async function connectToServer(address: string): Promise<string> {
  return invoke<string>('connect_to_server', { address });
}

// --- Campaigns ---

export async function listCampaigns(): Promise<Campaign[]> {
  return invokeJson<Campaign[]>('list_campaigns');
}

export async function getCampaign(id: string): Promise<Campaign> {
  return invokeJson<Campaign>('get_campaign', { id });
}

export async function createCampaign(name: string, description: string): Promise<Campaign> {
  return invokeJson<Campaign>('create_campaign', { name, description });
}

export async function updateCampaign(
  id: string,
  name: string,
  description: string,
  status: string,
): Promise<Campaign> {
  return invokeJson<Campaign>('update_campaign', { id, name, description, status });
}

// --- Implants ---

export async function listImplants(): Promise<Implant[]> {
  return invokeJson<Implant[]>('list_implants');
}

export async function getImplant(id: string): Promise<Implant> {
  return invokeJson<Implant>('get_implant', { id });
}

export async function killImplant(implantId: string): Promise<void> {
  await invoke('kill_implant', { implantId });
}

// --- Listeners ---

export async function listListeners(): Promise<Listener[]> {
  return invokeJson<Listener[]>('list_listeners');
}

export async function createListener(
  name: string,
  type_: string,
  bindAddress: string,
  port: number,
): Promise<Listener> {
  return invokeJson<Listener>('create_listener', { name, type_, bindAddress, port });
}

export async function startListener(listenerId: string): Promise<void> {
  await invoke('start_listener', { listenerId });
}

export async function stopListener(listenerId: string): Promise<void> {
  await invoke('stop_listener', { listenerId });
}

// --- Commands ---

export async function sendCommand(
  implantId: string,
  commandType: string,
  payload: string,
): Promise<string> {
  return invoke<string>('send_command', { implantId, commandType, payload });
}

export async function listCommands(implantId: string): Promise<Command[]> {
  return invokeJson<Command[]>('list_commands', { implantId });
}

export async function getCommandResult(commandId: string): Promise<CommandResult> {
  return invokeJson<CommandResult>('get_command_result', { commandId });
}

export async function cancelCommand(commandId: string): Promise<void> {
  await invoke('cancel_command', { commandId });
}

// --- Credentials & Artifacts ---

export async function listCredentials(): Promise<Credential[]> {
  return invokeJson<Credential[]>('list_credentials');
}

export async function listArtifacts(): Promise<Artifact[]> {
  return invokeJson<Artifact[]>('list_artifacts');
}

export async function downloadArtifact(artifactId: string, savePath: string): Promise<string> {
  return invoke<string>('download_artifact', { artifactId, savePath });
}

// --- Persistence ---

export async function listPersistence(implantId: string): Promise<PersistenceItem[]> {
  return invokeJson<PersistenceItem[]>('list_persistence', { implantId });
}

export async function removePersistence(id: string): Promise<void> {
  await invoke('remove_persistence', { id });
}

// --- Attack Chains ---

export async function createAttackChain(
  name: string,
  description: string,
  steps: ChainStepInput[],
): Promise<AttackChain> {
  return invokeJson<AttackChain>('create_attack_chain', { name, description, steps });
}

export async function listAttackChains(): Promise<AttackChain[]> {
  return invokeJson<AttackChain[]>('list_attack_chains');
}

export async function getAttackChain(id: string): Promise<AttackChain> {
  return invokeJson<AttackChain>('get_attack_chain', { id });
}

export async function executeAttackChain(chainId: string, implantId: string): Promise<void> {
  await invoke('execute_attack_chain', { chainId, implantId });
}

// --- Playbooks ---

export async function listPlaybooks(): Promise<Playbook[]> {
  return invokeJson<Playbook[]>('list_playbooks');
}

export async function instantiatePlaybook(
  playbookId: string,
  nameOverride: string,
): Promise<AttackChain> {
  return invokeJson<AttackChain>('instantiate_playbook', { playbookId, nameOverride });
}

// --- Implant Generator ---

export async function generateImplant(
  platform: string,
  arch: string,
  format: string,
  c2Url: string,
  sleepInterval: number,
  savePath: string,
): Promise<string> {
  return invoke<string>('generate_implant', {
    platform,
    arch,
    format,
    c2Url,
    sleepInterval,
    savePath,
  });
}

// --- Phishing ---

export async function createPhishing(
  type_: string,
  c2Url: string,
  savePath: string,
  method?: string,
): Promise<string> {
  return invoke<string>('create_phishing', { type_, c2Url, savePath, method });
}

// --- PowerShell Profile ---

export async function setPowershellProfile(
  implantId: string,
  profileScript: string,
): Promise<void> {
  await invoke('set_powershell_profile', { implantId, profileScript });
}

export async function getPowershellProfile(implantId: string): Promise<string> {
  return invoke<string>('get_powershell_profile', { implantId });
}

// --- Auth ---

export async function refreshToken(token: string): Promise<string> {
  return invoke<string>('refresh_token', { token });
}

// --- Events ---

export async function streamEvents(): Promise<void> {
  await invoke('stream_events');
}
