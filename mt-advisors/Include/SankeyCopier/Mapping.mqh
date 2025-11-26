//+------------------------------------------------------------------+
//|                                       SankeyCopierMapping.mqh    |
//|                            Sankey Copier Ticket Mapping Utilities |
//|                                                                  |
//| Purpose: Provides unified ticket mapping management for Slave EA|
//|          Eliminates code duplication between MT4 and MT5 versions|
//|          Manages master-slave ticket associations               |
//|          Supports restart recovery via position comments         |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SANKEY_COPIER_MAPPING_MQH
#define SANKEY_COPIER_MAPPING_MQH

#include "Common.mqh"

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

// Ticket mapping structure for regular orders/positions
struct TicketMapping
{
   TICKET_TYPE master_ticket;
   TICKET_TYPE slave_ticket;
};

// Ticket mapping structure for pending orders
struct PendingTicketMapping
{
   TICKET_TYPE master_ticket;
   TICKET_TYPE pending_ticket;
};

//+------------------------------------------------------------------+
//| Add master-slave ticket mapping                                  |
//| Parameters:                                                       |
//|   map            - Mapping array                                 |
//|   master_ticket  - Master EA ticket number                       |
//|   slave_ticket   - Slave EA ticket number                        |
//+------------------------------------------------------------------+
void AddTicketMapping(TicketMapping &map[], TICKET_TYPE master_ticket, TICKET_TYPE slave_ticket)
{
   int size = ArraySize(map);
   ArrayResize(map, size + 1);
   map[size].master_ticket = master_ticket;
   map[size].slave_ticket = slave_ticket;
}

//+------------------------------------------------------------------+
//| Get slave ticket from master ticket                              |
//| Parameters:                                                       |
//|   map            - Mapping array                                 |
//|   master_ticket  - Master EA ticket number to look up            |
//| Returns: Slave ticket if found, 0 if not found                  |
//+------------------------------------------------------------------+
TICKET_TYPE GetSlaveTicketFromMapping(TicketMapping &map[], TICKET_TYPE master_ticket)
{
   for(int i = 0; i < ArraySize(map); i++)
   {
      if(map[i].master_ticket == master_ticket)
         return map[i].slave_ticket;
   }
   return 0;
}

//+------------------------------------------------------------------+
//| Remove ticket mapping by master ticket                           |
//| Parameters:                                                       |
//|   map            - Mapping array                                 |
//|   master_ticket  - Master EA ticket number to remove             |
//+------------------------------------------------------------------+
void RemoveTicketMapping(TicketMapping &map[], TICKET_TYPE master_ticket)
{
   for(int i = 0; i < ArraySize(map); i++)
   {
      if(map[i].master_ticket == master_ticket)
      {
         // Shift array elements
         for(int j = i; j < ArraySize(map) - 1; j++)
         {
            map[j] = map[j + 1];
         }
         ArrayResize(map, ArraySize(map) - 1);
         break;
      }
   }
}

//+------------------------------------------------------------------+
//| Add pending order mapping                                        |
//| Parameters:                                                       |
//|   map            - Pending mapping array                         |
//|   master_ticket  - Master EA ticket number                       |
//|   pending_ticket - Pending order ticket number                   |
//+------------------------------------------------------------------+
void AddPendingTicketMapping(PendingTicketMapping &map[], TICKET_TYPE master_ticket, TICKET_TYPE pending_ticket)
{
   int size = ArraySize(map);
   ArrayResize(map, size + 1);
   map[size].master_ticket = master_ticket;
   map[size].pending_ticket = pending_ticket;
}

//+------------------------------------------------------------------+
//| Get pending ticket from master ticket                            |
//| Parameters:                                                       |
//|   map            - Pending mapping array                         |
//|   master_ticket  - Master EA ticket number to look up            |
//| Returns: Pending ticket if found, 0 if not found                |
//+------------------------------------------------------------------+
TICKET_TYPE GetPendingTicketFromMapping(PendingTicketMapping &map[], TICKET_TYPE master_ticket)
{
   for(int i = 0; i < ArraySize(map); i++)
   {
      if(map[i].master_ticket == master_ticket)
         return map[i].pending_ticket;
   }
   return 0;
}

//+------------------------------------------------------------------+
//| Remove pending ticket mapping by master ticket                   |
//| Parameters:                                                       |
//|   map            - Pending mapping array                         |
//|   master_ticket  - Master EA ticket number to remove             |
//+------------------------------------------------------------------+
void RemovePendingTicketMapping(PendingTicketMapping &map[], TICKET_TYPE master_ticket)
{
   for(int i = 0; i < ArraySize(map); i++)
   {
      if(map[i].master_ticket == master_ticket)
      {
         // Shift array elements
         for(int j = i; j < ArraySize(map) - 1; j++)
         {
            map[j] = map[j + 1];
         }
         ArrayResize(map, ArraySize(map) - 1);
         break;
      }
   }
}

//+------------------------------------------------------------------+
//| Transform symbol based on mappings                               |
//+------------------------------------------------------------------+
string TransformSymbol(string symbol, SymbolMapping &mappings[])
{
   for(int i=0; i<ArraySize(mappings); i++)
   {
      if(mappings[i].source_symbol == symbol)
         return mappings[i].target_symbol;
   }
   return symbol;
}

// =============================================================================
// Comment Format Helpers
// =============================================================================

//+------------------------------------------------------------------+
//| Build comment string for market position                          |
//| Format: "M{master_ticket}" (e.g., "M1234567890")                  |
//| Returns: Comment string, max 21 chars                            |
//+------------------------------------------------------------------+
string BuildMarketComment(TICKET_TYPE master_ticket)
{
   return COMMENT_PREFIX_MARKET + IntegerToString(master_ticket);
}

//+------------------------------------------------------------------+
//| Build comment string for pending order                            |
//| Format: "P{master_ticket}" (e.g., "P1234567890")                  |
//| Returns: Comment string, max 21 chars                            |
//+------------------------------------------------------------------+
string BuildPendingComment(TICKET_TYPE master_ticket)
{
   return COMMENT_PREFIX_PENDING + IntegerToString(master_ticket);
}

//+------------------------------------------------------------------+
//| Parse master ticket from comment string                           |
//| Input: Comment string (e.g., "M1234567890" or "P1234567890")      |
//| Output: Master ticket number, 0 if invalid format                |
//| Output: is_pending set to true if pending order comment          |
//+------------------------------------------------------------------+
TICKET_TYPE ParseMasterTicketFromComment(string comment, bool &is_pending)
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

   TICKET_TYPE ticket = (TICKET_TYPE)StringToInteger(ticket_str);
   return ticket;
}

// =============================================================================
// Startup Recovery Functions
// =============================================================================

//+------------------------------------------------------------------+
//| Recover ticket mappings from existing positions (MT5)             |
//| Call this on EA startup to restore mappings after restart        |
//| Parameters:                                                       |
//|   map         - Market position mapping array to populate        |
//|   pending_map - Pending order mapping array to populate          |
//| Returns: Number of mappings recovered                            |
//+------------------------------------------------------------------+
#ifdef IS_MT5
int RecoverMappingsFromPositions(TicketMapping &map[], PendingTicketMapping &pending_map[])
{
   int recovered_count = 0;

   // Clear existing mappings
   ArrayResize(map, 0);
   ArrayResize(pending_map, 0);

   Print("=== Recovering ticket mappings from existing positions ===");

   // Scan all open positions
   int pos_total = PositionsTotal();
   for(int i = 0; i < pos_total; i++)
   {
      ulong ticket = PositionGetTicket(i);
      if(ticket == 0) continue;

      if(!PositionSelectByTicket(ticket)) continue;

      string comment = PositionGetString(POSITION_COMMENT);
      bool is_pending = false;
      TICKET_TYPE master_ticket = ParseMasterTicketFromComment(comment, is_pending);

      if(master_ticket > 0 && !is_pending)
      {
         // This is a position we copied from master
         AddTicketMapping(map, master_ticket, ticket);
         recovered_count++;
         Print("Recovered mapping: master #", master_ticket, " -> slave #", ticket, " (comment: ", comment, ")");
      }
   }

   // Scan all pending orders
   int order_total = OrdersTotal();
   for(int i = 0; i < order_total; i++)
   {
      ulong ticket = OrderGetTicket(i);
      if(ticket == 0) continue;

      if(!OrderSelect(ticket)) continue;

      string comment = OrderGetString(ORDER_COMMENT);
      bool is_pending = false;
      TICKET_TYPE master_ticket = ParseMasterTicketFromComment(comment, is_pending);

      if(master_ticket > 0 && is_pending)
      {
         // This is a pending order we created for delayed signal
         AddPendingTicketMapping(pending_map, master_ticket, ticket);
         recovered_count++;
         Print("Recovered pending mapping: master #", master_ticket, " -> pending #", ticket, " (comment: ", comment, ")");
      }
   }

   Print("=== Recovery complete: ", recovered_count, " mappings restored ===");
   return recovered_count;
}
#endif

//+------------------------------------------------------------------+
//| Recover ticket mappings from existing positions (MT4)             |
//| Call this on EA startup to restore mappings after restart        |
//| Parameters:                                                       |
//|   map         - Market position mapping array to populate        |
//|   pending_map - Pending order mapping array to populate          |
//| Returns: Number of mappings recovered                            |
//+------------------------------------------------------------------+
#ifdef IS_MT4
int RecoverMappingsFromPositions(TicketMapping &map[], PendingTicketMapping &pending_map[])
{
   int recovered_count = 0;

   // Clear existing mappings
   ArrayResize(map, 0);
   ArrayResize(pending_map, 0);

   Print("=== Recovering ticket mappings from existing positions ===");

   // In MT4, both positions and pending orders are in OrdersTotal()
   int total = OrdersTotal();
   for(int i = 0; i < total; i++)
   {
      if(!OrderSelect(i, SELECT_BY_POS, MODE_TRADES)) continue;

      int ticket = OrderTicket();
      string comment = OrderComment();
      int order_type = OrderType();

      bool is_pending = false;
      TICKET_TYPE master_ticket = ParseMasterTicketFromComment(comment, is_pending);

      if(master_ticket > 0)
      {
         if(order_type == OP_BUY || order_type == OP_SELL)
         {
            // Market position
            if(!is_pending)
            {
               AddTicketMapping(map, master_ticket, ticket);
               recovered_count++;
               Print("Recovered mapping: master #", master_ticket, " -> slave #", ticket);
            }
         }
         else
         {
            // Pending order (OP_BUYLIMIT, OP_SELLLIMIT, etc.)
            if(is_pending)
            {
               AddPendingTicketMapping(pending_map, master_ticket, ticket);
               recovered_count++;
               Print("Recovered pending mapping: master #", master_ticket, " -> pending #", ticket);
            }
         }
      }
   }

   Print("=== Recovery complete: ", recovered_count, " mappings restored ===");
   return recovered_count;
}
#endif

#endif // SANKEY_COPIER_MAPPING_MQH
