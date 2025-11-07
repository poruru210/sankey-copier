//+------------------------------------------------------------------+
//|                                     ForexCopierMessages.mqh      |
//|                        Copyright 2025, Forex Copier Project      |
//|                     Message sending utilities                     |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"

#include "ForexCopierCommon.mqh"
#include "ForexCopierJson.mqh"

//+------------------------------------------------------------------+
//| Send registration message to server                              |
//+------------------------------------------------------------------+
bool SendRegistrationMessage(int zmq_context, string server_address, string account_id, string ea_type, string platform)
{
   // Create temporary PUSH socket for registration
   int push_socket = zmq_socket_create(zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      Print("ERROR: Failed to create registration socket");
      return false;
   }

   if(zmq_socket_connect(push_socket, server_address) == 0)
   {
      Print("ERROR: Failed to connect to registration server: ", server_address);
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Build registration message using JSON builder
   CJsonBuilder json;
   json.AddString("message_type", "Register");
   json.AddString("account_id", account_id);
   json.AddString("ea_type", ea_type);
   json.AddString("platform", platform);
   json.AddInteger("account_number", GetAccountNumber());
   json.AddString("broker", GetBrokerName());
   json.AddString("account_name", GetAccountName());
   json.AddString("server", GetServerName());
   json.AddNumber("balance", GetAccountBalance(), 2);
   json.AddNumber("equity", GetAccountEquity(), 2);
   json.AddString("currency", GetAccountCurrency());
   json.AddInteger("leverage", GetAccountLeverage());
   json.AddString("timestamp", FormatTimestampISO8601(TimeCurrent()));

   string message = json.ToString();
   bool success = (zmq_socket_send(push_socket, message) == 1);

   if(success)
      Print("Registration message sent successfully");
   else
      Print("ERROR: Failed to send registration message");

   zmq_socket_destroy(push_socket);
   return success;
}

//+------------------------------------------------------------------+
//| Send unregistration message to server                            |
//+------------------------------------------------------------------+
bool SendUnregistrationMessage(int zmq_context, string server_address, string account_id)
{
   // Create temporary PUSH socket for unregistration
   int push_socket = zmq_socket_create(zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      Print("ERROR: Failed to create unregistration socket");
      return false;
   }

   if(zmq_socket_connect(push_socket, server_address) == 0)
   {
      Print("ERROR: Failed to connect to unregistration server: ", server_address);
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Build unregistration message
   CJsonBuilder json;
   json.AddString("message_type", "Unregister");
   json.AddString("account_id", account_id);
   json.AddString("timestamp", FormatTimestampISO8601(TimeCurrent()));

   string message = json.ToString();
   bool success = (zmq_socket_send(push_socket, message) == 1);

   if(success)
      Print("Unregistration message sent successfully");
   else
      Print("ERROR: Failed to send unregistration message");

   zmq_socket_destroy(push_socket);
   return success;
}

//+------------------------------------------------------------------+
//| Send heartbeat message to server                                 |
//+------------------------------------------------------------------+
bool SendHeartbeatMessage(int zmq_context, string server_address, string account_id)
{
   // Create temporary PUSH socket for heartbeat
   int push_socket = zmq_socket_create(zmq_context, ZMQ_PUSH);
   if(push_socket < 0)
   {
      Print("ERROR: Failed to create heartbeat socket");
      return false;
   }

   if(zmq_socket_connect(push_socket, server_address) == 0)
   {
      Print("ERROR: Failed to connect to heartbeat server: ", server_address);
      zmq_socket_destroy(push_socket);
      return false;
   }

   // Build heartbeat message
   CJsonBuilder json;
   json.AddString("message_type", "Heartbeat");
   json.AddString("account_id", account_id);
   json.AddNumber("balance", GetAccountBalance(), 2);
   json.AddNumber("equity", GetAccountEquity(), 2);
   json.AddInteger("open_positions", GetOpenPositionsCount());
   json.AddString("timestamp", FormatTimestampISO8601(TimeCurrent()));

   string message = json.ToString();
   bool success = (zmq_socket_send(push_socket, message) == 1);

   zmq_socket_destroy(push_socket);
   return success;
}

//+------------------------------------------------------------------+
//| Send trade signal message (Master)                               |
//+------------------------------------------------------------------+
bool SendTradeSignal(int zmq_socket, string action, TICKET_TYPE ticket, string symbol,
                     string order_type, double lots, double price, double sl, double tp,
                     long magic, string comment, string account_id)
{
   CJsonBuilder json;
   json.AddString("action", action);
   json.AddInteger("ticket", (long)ticket);
   json.AddString("symbol", symbol);
   json.AddString("order_type", order_type);
   json.AddNumber("lots", lots, 2);
   json.AddNumber("open_price", price, 5);
   json.AddNumberOrNull("stop_loss", sl, 5);
   json.AddNumberOrNull("take_profit", tp, 5);
   json.AddInteger("magic_number", magic);
   json.AddString("comment", comment);
   json.AddString("timestamp", FormatTimestampISO8601(TimeCurrent()));
   json.AddString("source_account", account_id);

   string message = json.ToString();
   return (zmq_socket_send(zmq_socket, message) == 1);
}

//+------------------------------------------------------------------+
//| Send close signal message (Master)                               |
//+------------------------------------------------------------------+
bool SendCloseSignal(int zmq_socket, TICKET_TYPE ticket, string account_id)
{
   CJsonBuilder json;
   json.AddString("action", "Close");
   json.AddInteger("ticket", (long)ticket);
   json.AddString("timestamp", FormatTimestampISO8601(TimeCurrent()));
   json.AddString("source_account", account_id);

   string message = json.ToString();
   return (zmq_socket_send(zmq_socket, message) == 1);
}
