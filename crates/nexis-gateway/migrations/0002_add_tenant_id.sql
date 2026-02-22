-- Add tenant_id column to rooms table
ALTER TABLE rooms ADD COLUMN IF NOT EXISTS tenant_id TEXT;

-- Add tenant_id column to messages table
ALTER TABLE messages ADD COLUMN IF NOT EXISTS tenant_id TEXT;

-- Add tenant_id column to members table
ALTER TABLE members ADD COLUMN IF NOT EXISTS tenant_id TEXT;

-- Create composite indexes for tenant-aware queries
CREATE INDEX IF NOT EXISTS idx_rooms_tenant_id_id ON rooms (tenant_id, id);
CREATE INDEX IF NOT EXISTS idx_messages_tenant_id_id ON messages (tenant_id, id);
CREATE INDEX IF NOT EXISTS idx_members_tenant_id_id ON members (tenant_id, id);

-- Create index for tenant-scoped room message lookups
CREATE INDEX IF NOT EXISTS idx_messages_tenant_id_room_id ON messages (tenant_id, room_id);
