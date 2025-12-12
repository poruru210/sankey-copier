//+------------------------------------------------------------------+
//|                                       SankeyCopierMapping.mqh    |
//|                            Sankey Copier Ticket Mapping Utilities |
//|                                                                  |
//| Purpose: Provides utilities for comment parsing and formatting.   |
//|          (Legacy mapping array functions removed in favor of FFI) |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SANKEY_COPIER_MAPPING_MQH
#define SANKEY_COPIER_MAPPING_MQH

#include "Common.mqh"
#include "Logging.mqh"

// Platform detection
#ifndef IS_MT4
#ifndef IS_MT5
   #ifdef __MQL5__
      #define IS_MT5
      #define TICKET_TYPE ulong
   #else
      #define IS_MT4
      #define TICKET_TYPE int
   #endif
#endif
#endif

// =============================================================================
// Comment Format Constants
// =============================================================================
// Comment format for tracking master-slave position relationship
// Format: "{PREFIX}{master_ticket}"
// Example: "M1234567890" for market position, "P1234567890" for pending order
// Max length: 21 chars (1 prefix + 20 digit max ulong)
// Note: MT5 comment limit is 31 chars, so this format is safe

#define COMMENT_PREFIX_MARKET   "M"    // Prefix for market positions
#define COMMENT_PREFIX_PENDING  "P"    // Prefix for pending orders

// =============================================================================
// Comment Format Helpers
// =============================================================================

//+------------------------------------------------------------------+
//| Build comment string for market position                          |
//| Format: "M{master_ticket}" (e.g., "M1234567890")                  |
//| Returns: Comment string, max 21 chars                            |
//+------------------------------------------------------------------+
string BuildMarketComment(long master_ticket)
{
   return COMMENT_PREFIX_MARKET + IntegerToString(master_ticket);
}

//+------------------------------------------------------------------+
//| Build comment string for pending order                            |
//| Format: "P{master_ticket}" (e.g., "P1234567890")                  |
//| Returns: Comment string, max 21 chars                            |
//+------------------------------------------------------------------+
string BuildPendingComment(long master_ticket)
{
   return COMMENT_PREFIX_PENDING + IntegerToString(master_ticket);
}

//+------------------------------------------------------------------+
//| Parse master ticket from comment string                           |
//| Input: Comment string (e.g., "M1234567890" or "P1234567890")      |
//| Output: Master ticket number, 0 if invalid format                |
//| Output: is_pending set to true if pending order comment          |
//+------------------------------------------------------------------+
long ParseMasterTicketFromComment(string comment, bool &is_pending)
{
   is_pending = false;

   if(StringLen(comment) < 2)
      return 0;

   string prefix = StringSubstr(comment, 0, 1);

   if(prefix == COMMENT_PREFIX_MARKET)
   {
      is_pending = false;
   }
   else if(prefix == COMMENT_PREFIX_PENDING)
   {
      is_pending = true;
   }
   else
   {
      // Not our comment format
      return 0;
   }

   // Extract ticket number after prefix
   string ticket_str = StringSubstr(comment, 1);

   // Handle case where broker appends text to comment (e.g., "M12345 [sl tp]")
   int space_pos = StringFind(ticket_str, " ");
   if(space_pos > 0)
   {
      ticket_str = StringSubstr(ticket_str, 0, space_pos);
   }

   // Also handle "#" separator from old format (e.g., "M12345#98765")
   int hash_pos = StringFind(ticket_str, "#");
   if(hash_pos > 0)
   {
      ticket_str = StringSubstr(ticket_str, 0, hash_pos);
   }

   return StringToInteger(ticket_str);
}

#endif // SANKEY_COPIER_MAPPING_MQH
