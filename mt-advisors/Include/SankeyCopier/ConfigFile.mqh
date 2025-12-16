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
//--- 2-port architecture: Receiver (PULL) and Publisher (unified PUB)
#define DEFAULT_RECEIVER_PORT    5555
#define DEFAULT_PUBLISHER_PORT   5556

//--- Global port variables (initialized from config file)
int g_ReceiverPort = DEFAULT_RECEIVER_PORT;
int g_PublisherPort = DEFAULT_PUBLISHER_PORT;
int g_PublisherPort = DEFAULT_PUBLISHER_PORT;
string g_SearchCandidates[]; // Array to store symbol search candidates
bool g_ConfigLoaded = false;

//+------------------------------------------------------------------+
//| Load configuration from INI file                                  |
//| Reads from terminal-specific MQL5/Files/ folder                   |
//| Uses FILE_ANSI flag because INI file is ASCII/UTF-8, not UTF-16   |
//| Returns: true if file was read, false if using defaults          |
//+------------------------------------------------------------------+
bool LoadConfig()
{
   if(g_ConfigLoaded)
      return true;

   // Check if config file exists
   if(!FileIsExist(CONFIG_FILENAME))
   {
      PrintFormat("[ConfigFile] Config file '%s' not found, using defaults: Receiver=%d, Publisher=%d",
                  CONFIG_FILENAME, g_ReceiverPort, g_PublisherPort);
      g_ConfigLoaded = true;
      return false;
   }

   // Open config file with FILE_ANSI flag
   // IMPORTANT: FILE_TXT without FILE_ANSI reads as UTF-16, causing garbage characters
   // The INI file is generated as ASCII/UTF-8 by Rust, so we must use FILE_ANSI
   int file_handle = FileOpen(CONFIG_FILENAME, FILE_READ | FILE_TXT | FILE_ANSI);
   if(file_handle == INVALID_HANDLE)
   {
      PrintFormat("[ConfigFile] Failed to open '%s' (error=%d), using defaults", CONFIG_FILENAME, GetLastError());
      g_ConfigLoaded = true;
      return false;
   }

   // Parse INI file
   bool in_zeromq_section = false;
   bool in_symbol_search_section = false;

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
         // Case-insensitive comparison for sections
         string upper_line = line;
         StringToUpper(upper_line);
         in_zeromq_section = (upper_line == "[ZEROMQ]");
         in_symbol_search_section = (upper_line == "[SYMBOLSEARCH]");
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

            // 2-port architecture: only ReceiverPort and PublisherPort
            if(key == "ReceiverPort")
               g_ReceiverPort = (int)StringToInteger(value);
            else if(key == "PublisherPort")
               g_PublisherPort = (int)StringToInteger(value);
         }
         }
      }
      // Parse [SymbolSearch] section
      else if(in_symbol_search_section)
      {
         int eq_pos = StringFind(line, "=");
         if(eq_pos > 0)
         {
            string key = StringSubstr(line, 0, eq_pos);
            string value = StringSubstr(line, eq_pos + 1);
            StringTrimLeft(key);
            StringTrimRight(key);
            
            if(key == "Candidates")
            {
               StringSplit(value, ',', g_SearchCandidates);
               // Trim loop
               for(int i=0; i<ArraySize(g_SearchCandidates); i++)
               {
                   StringTrimLeft(g_SearchCandidates[i]);
                   StringTrimRight(g_SearchCandidates[i]);
               }
            }
         }
      }
   }

   FileClose(file_handle);
   g_ConfigLoaded = true;

   PrintFormat("[ConfigFile] Loaded from '%s': Receiver=%d, Publisher=%d (unified)",
               CONFIG_FILENAME, g_ReceiverPort, g_PublisherPort);

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
//| 2-port architecture: same as Trade SUB (unified PUB socket)      |
//+------------------------------------------------------------------+
string GetConfigSubAddress()
{
   if(!g_ConfigLoaded)
      LoadConfig();
   // Both trade signals and configs come from the same unified PUB socket
   return StringFormat("tcp://localhost:%d", g_PublisherPort);
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
//| Get symbol search candidates                                      |
//+------------------------------------------------------------------+
void GetSymbolSearchCandidates(string &candidates[])
{
   if(!g_ConfigLoaded)
      LoadConfig();
   
   ArrayCopy(candidates, g_SearchCandidates);
}

//+------------------------------------------------------------------+
//| Check if config file exists                                       |
//+------------------------------------------------------------------+
bool ConfigFileExists()
{
   return FileIsExist(CONFIG_FILENAME);
}

//+------------------------------------------------------------------+
//| Reload configuration (force re-read from file)                   |
//+------------------------------------------------------------------+
void ReloadConfig()
{
   g_ConfigLoaded = false;
   g_ConfigLoaded = false;
   g_ReceiverPort = DEFAULT_RECEIVER_PORT;
   g_PublisherPort = DEFAULT_PUBLISHER_PORT;
   ArrayFree(g_SearchCandidates);
   LoadConfig();
}

#endif // SANKEY_COPIER_CONFIG_FILE_MQH
