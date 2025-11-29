//+------------------------------------------------------------------+
//|                                              ConfigFile.mqh      |
//|                        Copyright 2025, SANKEY Copier Project     |
//|                     Configuration file reader for ZMQ ports      |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"

#ifndef SANKEY_COPIER_CONFIG_FILE_MQH
#define SANKEY_COPIER_CONFIG_FILE_MQH

//--- Configuration file name (located in MQL5/Files/ or MQL4/Files/)
#define CONFIG_FILENAME "sankey_copier.ini"

//--- Default ports (fallback if config file not found)
#define DEFAULT_RECEIVER_PORT    5555
#define DEFAULT_PUBLISHER_PORT   5556
#define DEFAULT_CONFIG_PORT      5557

//--- Global port variables (initialized from config file)
int g_ReceiverPort = DEFAULT_RECEIVER_PORT;
int g_PublisherPort = DEFAULT_PUBLISHER_PORT;
int g_ConfigSenderPort = DEFAULT_CONFIG_PORT;
bool g_ConfigLoaded = false;

//+------------------------------------------------------------------+
//| Load configuration from INI file                                  |
//| Returns: true if file was read, false if using defaults          |
//+------------------------------------------------------------------+
bool LoadConfig()
{
   if(g_ConfigLoaded)
      return true;

   // Check if config file exists
   if(!FileIsExist(CONFIG_FILENAME, FILE_COMMON))
   {
      // Try without FILE_COMMON flag (local terminal folder)
      if(!FileIsExist(CONFIG_FILENAME))
      {
         PrintFormat("[ConfigFile] Config file '%s' not found, using defaults: Receiver=%d, Publisher=%d, Config=%d",
                     CONFIG_FILENAME, g_ReceiverPort, g_PublisherPort, g_ConfigSenderPort);
         g_ConfigLoaded = true;
         return false;
      }
   }

   // Try to open from common folder first, then local
   int file_handle = FileOpen(CONFIG_FILENAME, FILE_READ | FILE_TXT | FILE_COMMON);
   if(file_handle == INVALID_HANDLE)
   {
      file_handle = FileOpen(CONFIG_FILENAME, FILE_READ | FILE_TXT);
   }

   if(file_handle == INVALID_HANDLE)
   {
      PrintFormat("[ConfigFile] Failed to open '%s', using defaults", CONFIG_FILENAME);
      g_ConfigLoaded = true;
      return false;
   }

   // Parse INI file
   bool in_zeromq_section = false;

   while(!FileIsEnding(file_handle))
   {
      string line = FileReadString(file_handle);

      // Trim whitespace
      StringTrimLeft(line);
      StringTrimRight(line);

      // Skip empty lines and comments
      if(StringLen(line) == 0 || StringGetCharacter(line, 0) == '#')
         continue;

      // Check for section header
      if(StringGetCharacter(line, 0) == '[')
      {
         // Case-insensitive comparison for [ZeroMQ]
         string upper_line = line;
         StringToUpper(upper_line);
         in_zeromq_section = (upper_line == "[ZEROMQ]");
         continue;
      }

      // Parse key=value in [ZeroMQ] section
      if(in_zeromq_section)
      {
         int eq_pos = StringFind(line, "=");
         if(eq_pos > 0)
         {
            string key = StringSubstr(line, 0, eq_pos);
            string value = StringSubstr(line, eq_pos + 1);
            StringTrimLeft(key);
            StringTrimRight(key);
            StringTrimLeft(value);
            StringTrimRight(value);

            if(key == "ReceiverPort")
               g_ReceiverPort = (int)StringToInteger(value);
            else if(key == "PublisherPort")
               g_PublisherPort = (int)StringToInteger(value);
            else if(key == "ConfigSenderPort")
               g_ConfigSenderPort = (int)StringToInteger(value);
         }
      }
   }

   FileClose(file_handle);
   g_ConfigLoaded = true;

   PrintFormat("[ConfigFile] Loaded from '%s': Receiver=%d, Publisher=%d, Config=%d",
               CONFIG_FILENAME, g_ReceiverPort, g_PublisherPort, g_ConfigSenderPort);

   return true;
}

//+------------------------------------------------------------------+
//| Get PUSH socket address (EA -> Server)                           |
//+------------------------------------------------------------------+
string GetPushAddress()
{
   if(!g_ConfigLoaded)
      LoadConfig();
   return StringFormat("tcp://localhost:%d", g_ReceiverPort);
}

//+------------------------------------------------------------------+
//| Get Trade SUB socket address (Server -> EA)                      |
//+------------------------------------------------------------------+
string GetTradeSubAddress()
{
   if(!g_ConfigLoaded)
      LoadConfig();
   return StringFormat("tcp://localhost:%d", g_PublisherPort);
}

//+------------------------------------------------------------------+
//| Get Config SUB socket address (Server -> EA)                     |
//+------------------------------------------------------------------+
string GetConfigSubAddress()
{
   if(!g_ConfigLoaded)
      LoadConfig();
   return StringFormat("tcp://localhost:%d", g_ConfigSenderPort);
}

//+------------------------------------------------------------------+
//| Get receiver port                                                 |
//+------------------------------------------------------------------+
int GetReceiverPort()
{
   if(!g_ConfigLoaded)
      LoadConfig();
   return g_ReceiverPort;
}

//+------------------------------------------------------------------+
//| Get publisher port                                                |
//+------------------------------------------------------------------+
int GetPublisherPort()
{
   if(!g_ConfigLoaded)
      LoadConfig();
   return g_PublisherPort;
}

//+------------------------------------------------------------------+
//| Get config sender port                                            |
//+------------------------------------------------------------------+
int GetConfigSenderPort()
{
   if(!g_ConfigLoaded)
      LoadConfig();
   return g_ConfigSenderPort;
}

//+------------------------------------------------------------------+
//| Check if config file exists                                       |
//+------------------------------------------------------------------+
bool ConfigFileExists()
{
   return FileIsExist(CONFIG_FILENAME, FILE_COMMON) || FileIsExist(CONFIG_FILENAME);
}

//+------------------------------------------------------------------+
//| Reload configuration (force re-read from file)                   |
//+------------------------------------------------------------------+
void ReloadConfig()
{
   g_ConfigLoaded = false;
   g_ReceiverPort = DEFAULT_RECEIVER_PORT;
   g_PublisherPort = DEFAULT_PUBLISHER_PORT;
   g_ConfigSenderPort = DEFAULT_CONFIG_PORT;
   LoadConfig();
}

#endif // SANKEY_COPIER_CONFIG_FILE_MQH
