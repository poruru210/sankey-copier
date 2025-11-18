//+------------------------------------------------------------------+
//|                                            SankeyCopierZmq.mqh  |
//|                                  Sankey Copier ZeroMQ Utilities  |
//|                                                                  |
//| Purpose: Provides unified ZeroMQ initialization, connection,    |
//|          and cleanup functions for both MT4 and MT5 platforms.  |
//|          Eliminates code duplication across Master/Slave EAs.   |
//+------------------------------------------------------------------+
#property copyright "Sankey Copier"
#property strict

// Platform detection
#ifndef IS_MT4
#ifndef IS_MT5
   #ifdef __MQL5__
      #define IS_MT5
      #define HANDLE_TYPE long
   #else
      #define IS_MT4
      #define HANDLE_TYPE int
   #endif
#endif
#endif

// Note: ZMQ DLL functions are imported in SankeyCopierCommon.mqh
// Note: ZMQ socket type constants are defined in SankeyCopierCommon.mqh

//+------------------------------------------------------------------+
//| Initialize ZeroMQ context                                         |
//| Returns: Context handle on success, -1 on failure                |
//+------------------------------------------------------------------+
HANDLE_TYPE InitializeZmqContext()
{
   HANDLE_TYPE context = zmq_context_create();

   if(context < 0)
   {
      Print("ERROR: Failed to create ZMQ context");
      return -1;
   }

   Print("ZMQ context created successfully (handle: ", context, ")");
   return context;
}

//+------------------------------------------------------------------+
//| Create and connect ZeroMQ socket                                 |
//| Parameters:                                                       |
//|   context      - ZMQ context handle                              |
//|   socket_type  - Socket type (ZMQ_PUSH, ZMQ_SUB, etc.)          |
//|   address      - Connection address (e.g., "tcp://localhost:5555")|
//|   socket_name  - Descriptive name for logging                   |
//| Returns: Socket handle on success, -1 on failure                |
//+------------------------------------------------------------------+
HANDLE_TYPE CreateAndConnectZmqSocket(HANDLE_TYPE context, int socket_type, string address, string socket_name)
{
   // Validate context
   if(context < 0)
   {
      Print("ERROR: Invalid ZMQ context for ", socket_name);
      return -1;
   }

   // Create socket
   HANDLE_TYPE socket = zmq_socket_create(context, socket_type);
   if(socket < 0)
   {
      Print("ERROR: Failed to create ", socket_name, " socket (type: ", socket_type, ")");
      return -1;
   }

   Print(socket_name, " socket created (handle: ", socket, ")");

   // Connect to address
   if(zmq_socket_connect(socket, address) == 0)
   {
      Print("ERROR: Failed to connect ", socket_name, " to ", address);
      zmq_socket_destroy(socket);
      return -1;
   }

   Print(socket_name, " connected to ", address);
   return socket;
}

//+------------------------------------------------------------------+
//| Subscribe to topic on SUB socket                                 |
//| Parameters:                                                       |
//|   socket - SUB socket handle                                     |
//|   topic  - Topic to subscribe to (empty string for all messages)|
//| Returns: true on success, false on failure                      |
//+------------------------------------------------------------------+
bool SubscribeToTopic(HANDLE_TYPE socket, string topic)
{
   if(socket < 0)
   {
      Print("ERROR: Invalid socket handle for subscription");
      return false;
   }

   if(zmq_socket_subscribe(socket, topic) == 0)
   {
      Print("ERROR: Failed to subscribe to topic: ", (topic == "" ? "(all)" : topic));
      return false;
   }

   Print("Subscribed to topic: ", (topic == "" ? "(all messages)" : topic));
   return true;
}

//+------------------------------------------------------------------+
//| Cleanup ZeroMQ resources (socket and context)                    |
//| Parameters:                                                       |
//|   socket  - Socket handle (will be set to -1)                   |
//|   context - Context handle (will be set to -1)                  |
//| Note: Safe to call with -1 handles (will be skipped)            |
//+------------------------------------------------------------------+
void CleanupZmqSocket(HANDLE_TYPE &socket, string socket_name = "Socket")
{
   if(socket >= 0)
   {
      zmq_socket_destroy(socket);
      Print(socket_name, " destroyed");
      socket = -1;
   }
}

void CleanupZmqContext(HANDLE_TYPE &context)
{
   if(context >= 0)
   {
      zmq_context_destroy(context);
      Print("ZMQ context destroyed");
      context = -1;
   }
}

void CleanupZmqResources(HANDLE_TYPE &socket, HANDLE_TYPE &context, string socket_name = "Socket")
{
   CleanupZmqSocket(socket, socket_name);
   CleanupZmqContext(context);
}

//+------------------------------------------------------------------+
//| Cleanup multiple sockets and context                             |
//| For Slave EA with multiple sockets (trade + config)             |
//+------------------------------------------------------------------+
void CleanupZmqMultiSocket(HANDLE_TYPE &socket1, HANDLE_TYPE &socket2, HANDLE_TYPE &context,
                           string socket1_name = "Socket1", string socket2_name = "Socket2")
{
   CleanupZmqSocket(socket1, socket1_name);
   CleanupZmqSocket(socket2, socket2_name);
   CleanupZmqContext(context);
}
