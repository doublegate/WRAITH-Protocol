# The Dial-Up Frontier: An Exhaustive History of Automated Reconnaissance and Its Modern Evolution

## 1. Introduction: The Cartography of Connectivity

The history of information security is, at its fundamental core, a history of visibility. Before a system can be secured or exploited, its existence must be known. In the contemporary era of cloud computing and ubiquitous high-speed internet, the "network perimeter" is a well-defined concept, monitored by sophisticated firewalls and intrusion detection systems. However, in the formative decades of the digital age—spanning the 1970s through the 1990s—the perimeter was vast, invisible, and largely unguarded. It consisted of the Public Switched Telephone Network (PSTN), a global mesh of copper wires that connected homes and businesses to the nascent digital world. Within this analog expanse lay the gateways to the world’s most sensitive data: modems.

To map this invisible terrain, early explorers of the digital frontier developed a technique known as "wardialing." This process, which involved the systematic automated dialing of telephone numbers to identify responding modems, became the primary method of reconnaissance for hackers, spies, and security researchers alike. Wardialing was not merely a technical act; it was a cultural phenomenon that defined the ethos of the early hacker underground, influenced national security policy, and birthed the modern industry of vulnerability assessment.

This report provides an exhaustive analysis of the wardialing phenomenon. It traces the lineage of automated dialing from its innocuous origins in consumer electronics to its weaponization by the computer underground. It examines the technical mechanisms of the tools that powered this era—legendary software like ToneLoc and THC-Scan—and the legal battles that ensued as governments struggled to police a frontier they barely understood. Furthermore, this report bridges the temporal divide, demonstrating how the philosophy and methodology of wardialing have transmuted into the modern era. Today, the spirit of wardialing lives on in internet-scale scanners like Masscan and metadata search engines like Shodan, tools that perform the same function—brute-force discovery—on a global scale. By understanding the evolution of wardialing, we gain a profound insight into the trajectory of cybersecurity itself: a shift from the active scanning of analog lines to the passive indexing of digital assets.

---

## 2. The Analog Precursors: Telephony, Deregulation, and the "Demon Dialer" (1970s–1982)

To understand wardialing, one must first understand the environment in which it emerged. The 1970s telecommunications landscape in the United States was dominated by the Bell System (AT&T), a regulated monopoly that controlled not only the network infrastructure but also the devices attached to it. For decades, it was technically illegal to connect non-Bell equipment to the telephone network, a policy strictly enforced to protect the integrity of the grid. This closed ecosystem stifled innovation at the "edge" of the network, leaving the telephone instrument itself relatively primitive.

### 2.1 The Breakup of Bell and the Rise of Consumer Telephony
The legal and technical stranglehold of the Bell System began to loosen with the *Carterfone* decision of 1968, but it was the eventual court-ordered breakup of AT&T (initiated in 1974 and finalized in 1982, taking effect in 1984) that radically altered the landscape. The divestiture opened the market to third-party equipment manufacturers and alternative long-distance carriers like MCI and Sprint. This deregulation was the fertile soil from which automated dialing technology grew.

In the pre-divestiture era, long-distance calling was a simple, albeit expensive, affair handled entirely by AT&T. However, the emergence of competitors introduced complexity. To use a discount carrier like MCI in the late 1970s, a consumer could not simply dial "1" plus the area code. Instead, they had to dial a local access number (7 digits), wait for a second dial tone, enter a personal authorization code (often 5 to 7 digits), and then finally dial the destination number (10 digits). A single long-distance call could require dialing 24 or more digits. This friction created a market demand for "smart" telephones capable of automating the dialing process.

### 2.2 Zoom Telephonics and the Invention of the Demon Dialer
It was in this environment that Frank B. Manning and Bruce Kramer, two graduates of the Massachusetts Institute of Technology (MIT), founded Zoom Telephonics in 1977. The company initially produced "The Silencer," a switch to mute phone ringers, but their ambition lay in more sophisticated electronics. Drawing on an idea they had conceived as students to automate the reservation of tennis courts—which required calling precisely at noon when lines were busy—they developed a device that would become legendary: the **Demon Dialer**.

Released in 1980, the Demon Dialer was a standalone hardware peripheral that connected between the wall jack and the telephone. It was a marvel of the microprocessor era, capable of memorizing up to 176 telephone numbers and, crucially, handling the complex multi-stage dialing required by alternative carriers. However, its most notorious feature—and the one that gave the device its name—was its ability to handle busy signals.

When a user encountered a busy line, the Demon Dialer could be instructed to "demon dial" the number. The device would go off-hook, dial the number, listen for the busy signal, hang up, and immediately repeat the process, repeating this cycle up to 10 times a minute until the line cleared. Once the call connected, the device would signal the user to pick up the handset.

### 2.3 From Consumer Convenience to Hacker Tool
While Zoom Telephonics intended the Demon Dialer as a tool for convenience—saving fingers from fatigue and helping consumers save money on long-distance calls—the hacker and "phone phreak" communities immediately recognized its potential for other purposes.

In the lexicon of the early 1980s computer underground, "demon dialing" evolved from a noun describing a product to a verb describing a technique. Hackers used the technique to assault "modem pools." In this era, access to mainframe computers (like those at universities or large corporations) was often provided via a bank of modems connected to a "hunt group" of phone numbers. During peak hours, these lines were perpetually busy. A hacker would use a demon dialing script—or the hardware device itself—to hammer the hunt group number, seizing the first available line the millisecond a legitimate user disconnected.

This early iteration of the technique was distinct from what would later be called "wardialing." Demon dialing was an intensive attack against a *known* target (a specific busy number). It was a brute-force availability attack, ensuring the attacker was next in the queue. However, the conceptual leap from "dialing one number repeatedly" to "dialing many numbers sequentially" was imminent. As modems became more common, hackers realized that instead of fighting for access to a known computer, they could simply scan the vast expanse of the telephone network to find *unknown* computers that were sitting idle, waiting for a connection.

The hardware Demon Dialer itself became obsolete shortly after the Bell breakup was finalized. The implementation of "Equal Access" (1+ dialing) allowed consumers to choose a default long-distance carrier, eliminating the need for 24-digit dialing sequences. Sales of the device plummeted from $6 million to $1.5 million between 1984 and 1986. Yet, the term "demon dialing" persisted in the hacker lexicon, eventually becoming synonymous with the new, more aggressive technique of scanning entire exchanges.

---

## 3. The Cinematic Catalyst: *WarGames* and the Birth of Wardialing (1983)

If the Demon Dialer provided the mechanical precursor, Hollywood provided the cultural ignition. The release of the film *WarGames* in June 1983 is widely cited as the singular event that popularized the concept of automated scanning and introduced the term "wardialing" to the world. The film transformed a niche technical curiosity into a national security obsession and inspired a generation of teenagers to pick up modems.

### 3.1 The "WarGames" Scenario
The film’s protagonist, David Lightman (played by Matthew Broderick), is a prototypical early hacker: bright, bored, and equipped with an IMSAI 8080 microcomputer. In his quest to find the unlisted phone number of a computer game company in Sunnyvale, California, Lightman writes a program to dial every telephone number in the Sunnyvale exchange.

The scene depicting this process is iconic. Lightman’s computer systematically works through the numbers, effectively "hammering" the local telephone grid. When a number answers with a carrier tone (the screech of a modem), the computer logs the number. When a human answers, the computer politely disconnects. This is the definitive portrayal of wardialing: a brute-force sweep of a block of numbers to identify technological assets. Lightman eventually connects to a system he believes is the game company, but which turns out to be the WOPR (War Operation Plan Response), a NORAD supercomputer programmed to execute nuclear war simulations.

### 3.2 Terminology Shift: From Demon Dialing to Wardialing
Prior to *WarGames*, the technique was loosely referred to as "hammer dialing," "scanning," or "demon dialing". The movie crystallized the activity into a specific cultural trope. The term "wardialing" is a portmanteau of "WarGames" and "dialing." It is important to note that the characters in the movie do not use the term "wardialing"; the term was coined *by* the audience and the media to describe the technique shown *in* the film.

The impact of this rebranding cannot be overstated. "Demon dialing" sounded like a niche phreaking activity; "wardialing" sounded like a tactical operation. It lent a militaristic, aggressive cachet to the act of scanning. Software tools written in the wake of the movie, such as the "War Games Autodialer" for the Commodore 64, explicitly capitalized on this imagery.

### 3.3 The "WarGames Effect" on National Policy
The film’s release had tangible consequences for US national policy. President Ronald Reagan, who viewed the film at Camp David shortly after its release, was deeply unsettled by the premise. He reportedly paused a meeting with his Joint Chiefs of Staff to ask, "Could something like this really happen?" General John Vessey, Chairman of the Joint Chiefs, initially dismissed the film as fiction but returned a week later with a sobering report: "Mr. President, the problem is much worse than you think".

This realization—that the nation's critical infrastructure was accessible via the public telephone network—led directly to the signing of National Security Decision Directive 145 (NSDD-145) in 1984. This directive, titled "National Policy on Telecommunications and Automated Information Systems Security," was the first major federal effort to address computer security. It expanded the definition of sensitive information to include unclassified government data and tasked the NSA with securing federal computers. Thus, wardialing was not just a hacking technique; it was the catalyst for the modern information security state.

### 3.4 Technical Realism in Fiction
The *WarGames* depiction was surprisingly accurate for the time. The writers, Lawrence Lasker and Walter F. Parkes, had consulted with security experts like Willis Ware of the RAND Corporation and Peter Schwartz of the Stanford Research Institute. They correctly identified that while military computers might be secure, the *access points*—specifically maintenance modems left active for remote work—were the weak link. This vulnerability, known as the "backdoor modem," would remain the primary target of wardialers for the next two decades.

---

## 4. The Mechanics of the Attack: Anatomy of a Wardialing Campaign

To understand the threat posed by wardialing, one must understand the technical mechanics of the attack. In the 1980s and 90s, the telephone network was the internet. It was a circuit-switched network where a connection between two points was a physical electrical circuit.

### 4.1 The Target: The Exchange (NXX)
A telephone number in the North American Numbering Plan (NANP) consists of an Area Code (NPA), a Central Office Exchange (NXX), and a Subscriber Number (XXXX). For example, in `212-555-1234`, `212` is the NPA and `555` is the NXX.
A standard wardialing campaign targeted a specific exchange. An exchange contains 10,000 possible subscriber numbers (0000 through 9999). A hacker targeting a corporation located in the `555` exchange would program their wardialer to scan the entire 10,000-number block.

### 4.2 The Hardware: Modems and RS-232
The attacker required a personal computer (such as an Apple II, IBM PC, or Commodore 64) connected to a modem via a serial port (RS-232). The modem was the critical component. Early modems used acoustic couplers (where the phone handset was placed into rubber cups), but by the mid-80s, direct-connect modems were standard.
The communication between the computer and the modem was controlled by the Hayes Command Set (AT commands). A wardialing program is essentially a script that sends a sequence of AT commands to the modem to dial numbers, hang up, and interpret the results.

### 4.3 The Scanning Logic
A typical wardialing session followed this logic flow:

1.  **Initialization:** The software initializes the modem with a command string (e.g., `ATZ` to reset, `ATM0` to silence the speaker so the hacker can sleep while scanning).
2.  **Dialing:** The software sends `ATDT` (Attention Dial Tone) followed by the target number (e.g., `ATDT 555-0001`).
3.  **Monitoring:** The modem listens to the line. This is where the sophistication of the tool mattered.
    *   **Ring No Answer:** If the line rings for a specified duration (e.g., 30 seconds) without answer, the software sends a command to hang up (abort) and marks the number as "No Answer."
    *   **Busy Signal:** If a busy signal is detected, the number is marked "Busy" and usually queued for a retry later.
    *   **Voice Answer:** If a human answers ("Hello?"), or an answering machine picks up, the modem might not natively detect this as distinct from a carrier. Rudimentary wardialers relied on a timeout. Advanced modems (like US Robotics Courier) returned extended result codes that could differentiate voice energy, allowing the software to hang up instantly.
    *   **Carrier Detect:** If the remote device is a modem, it emits a high-pitched carrier tone (a handshake). The attacker's modem detects this frequency and sends a `CONNECT` result code to the computer.
4.  **Logging:** Upon receiving a `CONNECT` code, the software logs the number to a file (e.g., `FOUND.LOG` or `CARRIERS.DAT`).
5.  **Iteration:** The software increments the number (0001 to 0002) and repeats the process.

### 4.4 Fingerprinting the Hit
Once a modem was found, the second phase began: identification. A raw carrier tone tells the attacker nothing about the system behind it. It could be a bank mainframe, a university BBS, or a traffic light control system.
To identify the system, the attacker would connect to the identified number (often manually or using a "terminal" mode in the wardialer) and press `ENTER` a few times. The remote system would typically respond with a **banner**—a text string identifying the operating system or organization.
*   *Example Banner:* `UNIX System V Release 4.0 - Login:`
*   *Example Banner:* `Welcome to the First National Bank Private Network. Authorized Personnel Only.`
This phase, known as "banner grabbing," allowed hackers to prioritize targets. A banner identifying a VAX/VMS system might attract a different type of hacker than one identifying a Cisco router.

---

## 5. The Golden Age Tools: ToneLoc, THC-Scan, and PhoneSweep (1990–1999)

As the practice matured, the tools evolved from simple BASIC scripts to powerful, optimized applications. The 1990s saw the release of the definitive tools of the trade.

### 5.1 ToneLoc: The Legend
Released in the early 1990s by "Minor Threat" (Chris Lamprecht) and "Mucho Maas," **ToneLoc** (Tone Locator) is the most famous wardialing software ever written. Written for MS-DOS, it was a masterpiece of efficiency.
*   **Stealth via Randomization:** Sequential dialing (0000, 0001, 0002) was easily detected by the telephone company's Electronic Switching Systems (ESS). A sudden surge of sequential calls would trigger "mass calling" alarms. ToneLoc introduced randomized dialing, where the 10,000 numbers in an exchange were dialed in a random order. This distributed the traffic pattern, making it harder for the ESS to flag the activity as an attack.
*   **Extended Code Support:** ToneLoc was optimized for US Robotics modems, utilizing specific registers (`S-registers`) and return codes to detect dial tones and voice energy with high precision. This allowed for faster scanning; instead of waiting 60 seconds for a timeout, ToneLoc could hang up in 5 seconds if it detected a voice.
*   **Data Management:** ToneLoc managed its findings in `.dat` files, allowing scans to be paused, resumed, and merged. A full scan of an exchange could take days or weeks depending on the number of modems used; ToneLoc’s ability to manage this state was critical.

### 5.2 THC-Scan: The European Evolution
Developed by the German group "The Hacker's Choice" (THC), **THC-Scan** appeared in the mid-90s as a competitor to ToneLoc.
*   **Radix Dialing:** THC-Scan included algorithms specifically designed to defeat European PBX and exchange protections, which differed from the US Bell system.
*   **The "Boss Key":** Reflecting the demographic of its users (often teenagers or employees scanning from work), THC-Scan featured a "Boss Key." Pressing a single key would instantly hide the hacking interface and replace it with a harmless-looking DOS prompt or spreadsheet, allowing the hacker to feign innocence if interrupted.
*   **Statistics:** THC-Scan provided detailed statistical visualizations of the scan progress, appealing to the "war room" aesthetic of the hacker culture.

### 5.3 PhoneSweep: The Corporate Auditor
By 1998, the security industry had formalized wardialing as a legitimate component of a security audit. Sandstorm Enterprises released **PhoneSweep**, the first commercial, enterprise-grade wardialer.
*   **GUI and Reporting:** Unlike the command-line hacker tools, PhoneSweep ran on Windows with a graphical interface. It generated professional reports suitable for management, highlighting "rogue modems" that violated company policy.
*   **Fingerprinting Database:** PhoneSweep shipped with a massive database of over 470 known system banners. It could automatically identify a system as a "Cisco 2500 Router" or "Windows NT RAS" without manual intervention.
*   **Parallelism:** PhoneSweep supported multi-modem cards, allowing a security team to use 4, 8, or 16 phone lines simultaneously to scan an enterprise's vast phone ranges in a fraction of the time.

### 5.4 Feature Comparison of Historical Wardialers

| Feature | **ToneLoc** | **THC-Scan** | **PhoneSweep** |
| :--- | :--- | :--- | :--- |
| **Origin** | Minor Threat / Mucho Maas | The Hacker's Choice | Sandstorm Enterprises |
| **Era** | Early 1990s | Mid 1990s | Late 1990s |
| **Interface** | MS-DOS (TUI) | MS-DOS (TUI) | Windows (GUI) |
| **Primary User** | Hackers / Phreakers | Hackers / Hobbyists | Corporate Auditors |
| **Dialing Logic** | Randomized / Sequential | Radix / Randomized | Multi-line Parallel |
| **Detection Tech** | Modem Firmware (USR) | Audio Analysis | Banner Fingerprinting |
| **Cost** | Freeware | Freeware | Commercial License |

---

## 6. The Hacker Wars and the Law: LOD, MOD, and the CFAA (1990–1994)

Wardialing was the weapon of choice in the great "Hacker Wars" of the early 1990s, a conflict that pitted rival groups against each other and drew the full force of federal law enforcement.

### 6.1 LOD vs. MOD: Infrastructure Warfare
The **Legion of Doom (LOD)** and the **Masters of Deception (MOD)** were the two preeminent hacking groups of the era. LOD, based largely in Texas, and MOD, based in New York, engaged in a digital turf war for dominance of the X.25 networks and the telephone infrastructure.
Wardialing was their primary method of acquiring territory. Members of MOD used wardialers to scan the specific exchanges used by the Regional Bell Operating Companies (RBOCs) like NYNEX. Their goal was to find maintenance modems connected to the ESS switches themselves. Accessing these switches allowed them to control the phone network: rerouting calls, setting up wiretaps, and disconnecting rivals.

### 6.2 The Computer Fraud and Abuse Act (CFAA)
The legal response to this escalation was the **Computer Fraud and Abuse Act (CFAA)** of 1986 (amended in 1994 and later). The CFAA criminalized "unauthorized access" to a "protected computer".
*   **The Grey Area of Scanning:** Wardialing itself—the act of dialing and logging a carrier—occupied a legal grey area. Was dialing a number "access"? Most legal interpretations held that the crime occurred only when the hacker completed the handshake and attempted to log in. However, prosecutors in cases like *United States v. Riggs* (involving LOD members) often used wardialing logs as evidence of conspiracy and intent to commit fraud.
*   **State Statutes:** States like California enacted broader laws. **Penal Code 502** criminalized accessing a system "without permission," a definition broad enough to theoretically include the electronic "handshake" initiated by a wardialer.

### 6.3 Operation Sundevil and the SJG Raid
In 1990, the US Secret Service launched **Operation Sundevil**, a nationwide crackdown on credit card fraud and telephone abuse. The operation involved raids in 15 cities and the seizure of dozens of computers and BBS servers.
The most infamous raid targeted **Steve Jackson Games (SJG)** in Austin. The Secret Service believed that the company's upcoming game, *GURPS Cyberpunk*, was a manual for computer crime. The warrant was based on the fact that an SJG employee, Loyd Blankenship (The Mentor), ran a BBS where a stolen Bell South document (the E911 document) had been posted.
The E911 document described the administrative workings of the 911 system—information often gleaned through wardialing and social engineering. The Secret Service's inability to distinguish between a role-playing game manual and a hacking tool highlighted the government's profound technological illiteracy. The subsequent lawsuit, *Steve Jackson Games, Inc. v. United States Secret Service*, resulted in a victory for SJG and established that email stored on a BBS was protected by the Privacy Protection Act, a landmark ruling for digital rights.

---

## 7. The Digital Transition: WarVOX and the Shift to Wireless (2000–2010)

As the 21st century dawned, the PSTN began to recede as the primary carrier of data. Broadband internet (DSL, Cable) replaced dial-up. However, wardialing enjoyed a final, sophisticated renaissance before fading.

### 7.1 WarVOX: Audio Fingerprinting
In 2009, security researcher HD Moore (creator of the Metasploit Framework) released **WarVOX**. WarVOX was a wardialer for the broadband age. Instead of using a physical modem, it used Voice over IP (VoIP) providers (like Skype or SIP trunks) to make thousands of calls over the internet.
Critically, WarVOX did not just listen for a modem tone; it recorded the audio of every call and processed it using **Fast Fourier Transform (FFT)** algorithms. This technique, known as **audio fingerprinting**, allowed WarVOX to visually map the audio landscape of an organization.
*   **Beyond Modems:** WarVOX could identify specific voicemail systems (e.g., "This is a Cisco Unity system"), fax machines, and even specific IVR trees. It could group numbers based on the similarity of the audio, effectively mapping the internal structure of a corporation's PBX.
*   **Scale:** Because it used VoIP, WarVOX was not limited by the speed of a physical modem. It could scan 1,000 numbers per hour on a standard connection, significantly outpacing ToneLoc.

### 7.2 Wardriving: The Wireless Successor
As modems disappeared, the "wardialing" concept migrated to the wireless spectrum. **Wardriving** emerged as the direct successor.
*   **Concept:** Instead of dialing numbers to find a line, a hacker drives through a neighborhood with a laptop and a Wi-Fi card, scanning for Service Set Identifiers (SSIDs) of wireless access points.
*   **Parallels:** The methodology is identical—blind, sequential scanning of a medium to find unsecured entry points. Tools like **NetStumbler** and **Kismet** became the "ToneLoc" of the Wi-Fi era.
*   **Warchalking:** Inspired by "hobo signs," warchalkers would mark sidewalks with chalk symbols indicating the presence of open Wi-Fi nodes—a physical manifestation of the mapping process.

---

## 8. Modern Equivalents: Internet-Scale Asset Discovery (2010–Present)

Today, wardialing a phone number is largely a relic of the past. However, the *function* of wardialing—brute-force discovery of accessible assets—is more prevalent than ever. The medium has shifted from the 10,000-number telephone exchange to the 4.3 billion-address IPv4 space.

### 8.1 From Stateful to Stateless Scanning
The evolution of scanning tools mirrors the evolution of the network.

*   **Nmap (The Stateful Standard):** Nmap is the modern equivalent of a precision demon dialer. It is highly configurable, supports scripting (NSE), and is stealthy. However, it tracks the state of every connection (Stateful), which makes it relatively slow for massive scans.
*   **Masscan and ZMap (The Stateless Revolution):** To scan the entire internet, researchers developed **stateless scanners**. Tools like **Masscan** and **ZMap** do not maintain a TCP state table. They blast out SYN packets at line speed (up to 10 million packets per second) and listen for SYN-ACK responses asynchronously.
    *   *Benchmark:* Masscan can scan the entire internet (0.0.0.0/0) for a specific port in under 6 minutes. This is the industrialization of reconnaissance. Where ToneLoc took a week to scan a city, Masscan takes minutes to scan the planet.

### 8.2 The "Search Engines for Hackers": Shodan and Censys
The ultimate evolution of wardialing is **Shodan**. Launched in 2009, Shodan is a search engine that does not index web content (like Google) but indexes **service banners**.
*   **Pre-Computed Reconnaissance:** Shodan continuously "wardials" the entire internet, port scanning every IP address and storing the results. When a hacker wants to find a vulnerable system, they do not need to run a scanner; they simply query Shodan's database.
    *   *Query:* `port:3389 org:"Target Corp"`
    *   *Result:* A list of every Remote Desktop endpoint belonging to that corporation.
*   **Censys and FOFA:** Competitors like Censys (US) and FOFA (China) offer similar capabilities, with FOFA specializing in Asian networks and granular query syntax.

### 8.3 Comparative Analysis: The Evolution of Discovery

The following table illustrates the direct lineage from the analog tools of the 1980s to the cloud-scale engines of today.

| Era | **1980s (Analog)** | **2000s (Transition)** | **2020s (Digital)** |
| :--- | :--- | :--- | :--- |
| **Technique** | Wardialing | Wardriving / WarVOX | Internet Scanning |
| **Medium** | PSTN (Copper) | Wi-Fi (2.4GHz) / VoIP | TCP/IP (Fiber/Cloud) |
| **Address Space** | Exchange (10,000 nums) | Geolocation (Physical) | IPv4 (4 Billion addrs) |
| **Primary Tool** | ToneLoc | NetStumbler | Masscan / Shodan |
| **Target** | Unsecured Modem | Open Access Point | Misconfigured Cloud/IoT |
| **Identification** | Carrier Tone (2100Hz) | SSID / Beacon Frame | Service Banner / SSL Cert |
| **Speed** | ~1 call / 45 sec | ~500 nodes / hour | ~10M packets / sec |
| **Legal Risk** | High (Active dialing) | Medium (Passive sniff) | Low (Passive query) |

---

## 9. Conclusion: The End of Obscurity

The history of wardialing is the story of the death of "security by obscurity." In the 1970s, organizations believed their systems were secure simply because their phone numbers were unlisted. The Demon Dialer and ToneLoc shattered that illusion, proving that any connection that *could* be reached *would* be reached by a persistent enough automaton.

This lesson has been amplified a billion-fold in the internet age. Tools like Shodan and Masscan have rendered the concept of a "hidden" server obsolete. If a device is connected to the internet, it is being scanned, indexed, and cataloged within minutes of coming online. The modern equivalent of the "backdoor modem" is the unpatched IoT device or the exposed RDP port, and the modern wardialer is a botnet like Mirai, ceaselessly scouring the IP space for the digital equivalent of a carrier tone.

From the clicking relays of the Zoom Demon Dialer to the silent, light-speed packets of a Shodan crawler, the objective remains unchanged: to map the unknown. The tools have evolved, the laws have tightened, and the stakes have risen, but the scan continues.

---

## 10. Technical Appendix: Modulation and Detection

### 10.1 Common Modem Standards Detected by Wardialers
Wardialers differentiated targets based on the frequency of the answer tone.

| Standard | Baud Rate | Carrier Frequency | Application |
| :--- | :--- | :--- | :--- |
| **Bell 103** | 300 bps | 1070/1270 Hz | Early Hobbyist / BBS |
| **Bell 212A** | 1200 bps | 1200/2400 Hz | Standard 80s Data |
| **V.22bis** | 2400 bps | 1200/2400 Hz | Late 80s Standard |
| **V.32** | 9600 bps | 1800/2400 Hz | Early 90s High Speed |
| **Fax (G3)** | 14400 bps | 1100 Hz (CNG) | Facsimile Machines |

### 10.2 Wardialer Logic Pseudo-Code
The following pseudo-code represents the core logic loop of a tool like ToneLoc, illustrating the simplicity of the attack.

LOOP through Range (0000 to 9999):
  NUMBER = EXCHANGE + CURRENT_SUFFIX
  SEND "ATDT" + NUMBER to Modem
  WAIT for RESPONSE:
    IF RESPONSE == "CONNECT" THEN
      LOG "CARRIER FOUND" + NUMBER
      SEND "+++" (Escape Sequence)
      SEND "ATH0" (Hangup)
    ELSE IF RESPONSE == "NO CARRIER" THEN
      LOG "NO ANSWER"
    ELSE IF RESPONSE == "BUSY" THEN
      ADD NUMBER to RETRY_QUEUE
    ELSE IF TIME > 45_SECONDS THEN
      SEND "ATH0"
      LOG "TIMEOUT"
  INCREMENT CURRENT_SUFFIX
END LOOP

---

## Sources

*   **Blue Goat Cyber**. (n.d.). *What Is War Dialing?* Retrieved from [https://bluegoatcyber.com/blog/what-is-war-dialing/](https://bluegoatcyber.com/blog/what-is-war-dialing/)
*   **Computer Fraud and Abuse Act (CFAA)**. 18 U.S. Code § 1030. Retrieved from [https://en.wikipedia.org/wiki/Computer_Fraud_and_Abuse_Act](https://en.wikipedia.org/wiki/Computer_Fraud_and_Abuse_Act)
*   **EBSCO Research Starters**. (n.d.). *Demon Dialing/War Dialing*. Retrieved from [https://www.ebsco.com/research-starters/computer-science/demon-dialingwar-dialing](https://www.ebsco.com/research-starters/computer-science/demon-dialingwar-dialing)
*   **Enable Security**. (n.d.). *Attacking Real VoIP System with SIPVicious*. Retrieved from [https://www.enablesecurity.com/blog/attacking-real-voip-system-with-sipvicious-oss/](https://www.enablesecurity.com/blog/attacking-real-voip-system-with-sipvicious-oss/)
*   **FundingUniverse**. (n.d.). *Zoom Technologies, Inc. History*. Retrieved from [https://www.fundinguniverse.com/company-histories/zoom-technologies-inc-history/](https://www.fundinguniverse.com/company-histories/zoom-technologies-inc-history/)
*   **GIAC (SANS Institute)**. (2002). *War Dialing & War Driving Overview*. Retrieved from [https://www.giac.org/paper/gsec/863/war-dialing-war-driving-overview/101791](https://www.giac.org/paper/gsec/863/war-dialing-war-driving-overview/101791)
*   **Grokipedia**. (n.d.). *Wardialing*. Retrieved from(https://grokipedia.com/page/Wardialing)
*   **IT News**. (n.d.). *Wardialing - The Forgotten Front*. Retrieved from [https://www.itnews.com.au/feature/wardialing---the-forgotten-front-in-the-war-against-hackers-61310](https://www.itnews.com.au/feature/wardialing---the-forgotten-front-in-the-war-against-hackers-61310)
*   **MasterDC**. (n.d.). *What is Shodan Search Engine?* Retrieved from [https://www.masterdc.com/blog/what-is-shodan-search-engine/](https://www.masterdc.com/blog/what-is-shodan-search-engine/)
*   **Mitnick Security**. (n.d.). *Kevin Mitnick - The World's Most Famous Hacker*. Retrieved from [https://www.mitnicksecurity.com/about-kevin-mitnick](https://www.mitnicksecurity.com/about-kevin-mitnick)
*   **Nmap.org**. (n.d.). *Nmap Documentation*. Retrieved from [https://nmap.org/nmap_doc.html](https://nmap.org/nmap_doc.html)
*   **Palo Alto Networks**. (n.d.). *Revisiting WarGames*. Retrieved from [https://www.paloaltonetworks.com/perspectives/ctrl-alt-delusion-revisiting-wargames-42-years-later/](https://www.paloaltonetworks.com/perspectives/ctrl-alt-delusion-revisiting-wargames-42-years-later/)
*   **Phrack Magazine**. (n.d.). *Issue 43*. Retrieved from [https://phrack.org/issues/43/16](https://phrack.org/issues/43/16)
*   **Slatalla, M., & Quittner, J.** (1995). *Masters of Deception: The Gang That Ruled Cyberspace*. HarperCollins.
*   **Steve Jackson Games, Inc. v. United States Secret Service**. 816 F. Supp. 432 (W.D. Tex. 1993). Retrieved from(https://en.wikipedia.org/wiki/Steve_Jackson_Games,_Inc._v._United_States_Secret_Service)
*   **Twingate**. (2024). *What is a War Dialer?* Retrieved from [https://www.twingate.com/blog/glossary/war%20dialing](https://www.twingate.com/blog/glossary/war%20dialing)
*   **Wikipedia**. (n.d.). *Demon Dialing*. Retrieved from(https://en.wikipedia.org/wiki/Demon_dialing)
*   **Masscan**. (n.d.). *Masscan: Mass IP port scanner*. Retrieved from [https://github.com/robertdavidgraham/masscan](https://github.com/robertdavidgraham/masscan)
*   **Wikipedia**. (n.d.). *ToneLoc*. Retrieved from(https://en.wikipedia.org/wiki/ToneLoc)
*   **Wikipedia**. (n.d.). *WarGames*. Retrieved from(https://en.wikipedia.org/wiki/WarGames)
*   **Wikipedia**. (n.d.). *WarVOX*. Retrieved from(https://en.wikipedia.org/wiki/WarVOX)
*   **Zoom Telephonics**. (n.d.). *Company History*. Retrieved from(https://en.wikipedia.org/wiki/Zoom_Telephonics)
