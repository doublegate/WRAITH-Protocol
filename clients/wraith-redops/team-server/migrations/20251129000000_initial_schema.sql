-- Operators table
CREATE TABLE operators (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(64) UNIQUE NOT NULL,
    display_name VARCHAR(128),
    role VARCHAR(32) NOT NULL CHECK (role IN ('admin', 'operator', 'viewer')),
    public_key BYTEA NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_active TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT TRUE
);

-- Campaigns/Operations
CREATE TABLE campaigns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(128) NOT NULL,
    description TEXT,
    roe_document_id UUID,
    status VARCHAR(32) DEFAULT 'planning'
        CHECK (status IN ('planning', 'active', 'paused', 'completed', 'aborted')),
    start_date TIMESTAMPTZ,
    end_date TIMESTAMPTZ,
    created_by UUID REFERENCES operators(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Rules of Engagement documents
CREATE TABLE roe_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    campaign_id UUID REFERENCES campaigns(id),
    document JSONB NOT NULL,
    signature BYTEA NOT NULL,
    signing_key_id VARCHAR(64) NOT NULL,
    valid_from TIMESTAMPTZ NOT NULL,
    valid_until TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Implants
CREATE TABLE implants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    campaign_id UUID REFERENCES campaigns(id),
    hostname VARCHAR(255),
    internal_ip INET,
    external_ip INET,
    os_type VARCHAR(32),
    os_version VARCHAR(64),
    architecture VARCHAR(16),
    username VARCHAR(128),
    domain VARCHAR(255),
    privileges VARCHAR(32) CHECK (privileges IN ('user', 'admin', 'system')),
    implant_version VARCHAR(32),
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_checkin TIMESTAMPTZ,
    checkin_interval INTEGER,
    jitter_percent INTEGER,
    status VARCHAR(32) DEFAULT 'active'
        CHECK (status IN ('active', 'dormant', 'lost', 'killed')),
    notes TEXT,
    metadata JSONB
);

-- Implant network interfaces
CREATE TABLE implant_interfaces (
    id SERIAL PRIMARY KEY,
    implant_id UUID REFERENCES implants(id) ON DELETE CASCADE,
    interface_name VARCHAR(64),
    ip_address INET,
    mac_address MACADDR,
    is_primary BOOLEAN DEFAULT FALSE
);

-- Command queue
CREATE TABLE commands (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    implant_id UUID REFERENCES implants(id),
    operator_id UUID REFERENCES operators(id),
    command_type VARCHAR(64) NOT NULL,
    payload BYTEA,
    payload_encrypted BOOLEAN DEFAULT TRUE,
    priority INTEGER DEFAULT 5 CHECK (priority BETWEEN 1 AND 10),
    status VARCHAR(32) DEFAULT 'pending'
        CHECK (status IN ('pending', 'sent', 'received',
                          'executing', 'completed', 'failed', 'cancelled')),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    sent_at TIMESTAMPTZ,
    received_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    timeout_seconds INTEGER DEFAULT 300,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3
);

-- Command results
CREATE TABLE command_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    command_id UUID REFERENCES commands(id),
    output BYTEA,
    output_encrypted BOOLEAN DEFAULT TRUE,
    exit_code INTEGER,
    error_message TEXT,
    execution_time_ms INTEGER,
    received_at TIMESTAMPTZ DEFAULT NOW()
);

-- Collected files/artifacts
CREATE TABLE artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    implant_id UUID REFERENCES implants(id),
    command_id UUID REFERENCES commands(id),
    filename VARCHAR(512),
    original_path VARCHAR(1024),
    file_hash_sha256 BYTEA,
    file_hash_blake3 BYTEA,
    file_size BIGINT,
    mime_type VARCHAR(128),
    content BYTEA,
    collected_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB
);

-- Credentials
CREATE TABLE credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    implant_id UUID REFERENCES implants(id),
    source VARCHAR(64),
    credential_type VARCHAR(32)
        CHECK (credential_type IN ('password', 'hash', 'ticket', 'key', 'token')),
    domain VARCHAR(255),
    username VARCHAR(255),
    credential_data BYTEA,
    collected_at TIMESTAMPTZ DEFAULT NOW(),
    validated BOOLEAN,
    metadata JSONB
);

-- Activity log for audit
CREATE TABLE activity_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    operator_id UUID REFERENCES operators(id),
    implant_id UUID REFERENCES implants(id),
    campaign_id UUID REFERENCES campaigns(id),
    action VARCHAR(64) NOT NULL,
    details JSONB,
    source_ip INET,
    success BOOLEAN
);

-- Indexes for common queries
CREATE INDEX idx_implants_campaign ON implants(campaign_id);
CREATE INDEX idx_implants_status ON implants(status);
CREATE INDEX idx_implants_last_checkin ON implants(last_checkin);
CREATE INDEX idx_commands_implant ON commands(implant_id);
CREATE INDEX idx_commands_status ON commands(status);
CREATE INDEX idx_commands_created ON commands(created_at);
CREATE INDEX idx_activity_timestamp ON activity_log(timestamp);
CREATE INDEX idx_activity_campaign ON activity_log(campaign_id);
CREATE INDEX idx_credentials_domain_user ON credentials(domain, username);

-- Full-text search on activity details
CREATE INDEX idx_activity_details_gin ON activity_log USING GIN (details);
