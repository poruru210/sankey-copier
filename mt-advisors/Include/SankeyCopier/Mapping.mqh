//+------------------------------------------------------------------+
//|                                       SankeyCopierMapping.mqh    |
//|                            Sankey Copier Ticket Mapping Utilities |
//|                                                                  |
//| Purpose: Provides unified ticket mapping management for Slave EA|
//|          Eliminates code duplication between MT4 and MT5 versions|
//|          Manages master-slave ticket associations               |
//+------------------------------------------------------------------+
#property copyright "Sankey Copier"
#property strict

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
