# DLL Version Field Fix

## Issue

Slave EA heartbeats were failing with the following error:
```
Failed to deserialize Heartbeat message from EA [account_id: Tradexfin_Limited_75397602]: missing field `version`
```

## Root Cause

The installed DLL was compiled with an older version of the code that used the `version_git` field name. However, the server code was updated to expect the `version` field name (commit a89a6bf).

This mismatch caused the server to reject heartbeat messages from Slave EAs, preventing them from appearing in the web UI.

## Solution

1. **Rebuild DLL** with the latest code (build version: `0.1.0+build.285.d3a52d2`)
2. **Reinstall** to all MT installations via:
   - Copy to `C:\Program Files\SANKEY Copier\mql\` component directories
   - Use Installation Manager to deploy to all MT instances

## Verification

After the fix:
- Slave EA heartbeats are successfully deserialized
- Slave accounts appear correctly in web UI
- No more "missing field `version`" errors in server logs

## Prevention

To prevent this issue in the future:
- Always rebuild and redeploy DLLs after structural changes to message types
- Consider adding version compatibility checks in the protocol
- Document breaking changes in MessagePack structures

## Related Commits

- a89a6bf: Refactor: Rename version_git field to version
- d0a27a3: feat: Implement unified versioning system and consolidate build workflows
