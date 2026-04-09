-- Migration 006: version compatibility tracking for P2P peers
ALTER TABLE sync_state ADD COLUMN remote_version TEXT;
