CREATE TABLE persistence (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    implant_id UUID REFERENCES implants(id) ON DELETE CASCADE,
    method TEXT NOT NULL,
    details TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
