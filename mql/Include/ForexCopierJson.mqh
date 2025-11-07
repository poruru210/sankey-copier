//+------------------------------------------------------------------+
//|                                         ForexCopierJson.mqh      |
//|                        Copyright 2025, Forex Copier Project      |
//|                     JSON processing utilities                     |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, Forex Copier Project"

//+------------------------------------------------------------------+
//| JSON Builder - helps construct JSON strings                      |
//+------------------------------------------------------------------+
class CJsonBuilder
{
private:
   string m_json;
   bool   m_first_field;

public:
   CJsonBuilder() : m_json("{"), m_first_field(true) {}

   void AddString(string key, string value)
   {
      if(!m_first_field) m_json += ",";
      m_json += "\"" + key + "\":\"" + value + "\"";
      m_first_field = false;
   }

   void AddNumber(string key, double value, int digits = 2)
   {
      if(!m_first_field) m_json += ",";
      m_json += "\"" + key + "\":" + DoubleToString(value, digits);
      m_first_field = false;
   }

   void AddInteger(string key, long value)
   {
      if(!m_first_field) m_json += ",";
      m_json += "\"" + key + "\":" + IntegerToString(value);
      m_first_field = false;
   }

   void AddBool(string key, bool value)
   {
      if(!m_first_field) m_json += ",";
      m_json += "\"" + key + "\":" + (value ? "true" : "false");
      m_first_field = false;
   }

   void AddNull(string key)
   {
      if(!m_first_field) m_json += ",";
      m_json += "\"" + key + "\":null";
      m_first_field = false;
   }

   void AddNumberOrNull(string key, double value, int digits = 2)
   {
      if(value > 0)
         AddNumber(key, value, digits);
      else
         AddNull(key);
   }

   string ToString()
   {
      return m_json + "}";
   }
};

//+------------------------------------------------------------------+
//| Parse JSON value by key                                          |
//+------------------------------------------------------------------+
string GetJsonValue(string json, string key)
{
   string search = "\"" + key + "\":";
   int start = StringFind(json, search);
   if(start == -1) return "";

   start += StringLen(search);

   // Skip whitespace only (not quotes)
   int jsonLen = StringLen(json);
   while(start < jsonLen)
   {
      ushort c = StringGetCharacter(json, start);
      if(c != 32) break;  // 32 = space
      start++;
   }

   // Check if value starts with quote (string value)
   ushort firstChar = StringGetCharacter(json, start);
   bool isString = (firstChar == 34);  // 34 = double quote

   if(isString)
   {
      // Skip opening quote
      start++;

      // Find closing quote
      int end = start;
      while(end < jsonLen)
      {
         ushort c = StringGetCharacter(json, end);
         if(c == 34)  // Found closing quote
         {
            string value = StringSubstr(json, start, end - start);
            return value;
         }
         end++;
      }
      return "";  // No closing quote found
   }
   else
   {
      // Non-string value: find comma or closing brace
      int end = start;
      while(end < jsonLen)
      {
         ushort c = StringGetCharacter(json, end);
         if(c == 44 || c == 125) break;  // 44 = comma, 125 = }
         end++;
      }

      string value = StringSubstr(json, start, end - start);
      StringTrimLeft(value);
      StringTrimRight(value);
      return value;
   }
}

//+------------------------------------------------------------------+
//| Parse JSON string array                                          |
//+------------------------------------------------------------------+
void ParseStringArray(string json, string key, string &output[])
{
   ArrayResize(output, 0);

   string search = "\"" + key + "\":";
   int key_pos = StringFind(json, search);
   if(key_pos == -1) return;

   int array_start = StringFind(json, "[", key_pos);
   if(array_start == -1) return;

   int array_end = StringFind(json, "]", array_start);
   if(array_end == -1) return;

   string array_content = StringSubstr(json, array_start + 1, array_end - array_start - 1);
   StringTrimLeft(array_content);
   StringTrimRight(array_content);

   // Check for null or empty
   if(array_content == "" || StringFind(array_content, "null") == 0) return;

   // Split by comma (simple approach - assumes no commas in strings)
   string items[];
   int count = StringSplit(array_content, ',', items);

   for(int i = 0; i < count; i++)
   {
      string item = items[i];
      StringTrimLeft(item);
      StringTrimRight(item);
      StringReplace(item, "\"", "");

      if(item != "")
      {
         int idx = ArraySize(output);
         ArrayResize(output, idx + 1);
         output[idx] = item;
      }
   }
}

//+------------------------------------------------------------------+
//| Parse JSON int array                                             |
//+------------------------------------------------------------------+
void ParseIntArray(string json, string key, int &output[])
{
   ArrayResize(output, 0);

   string search = "\"" + key + "\":";
   int key_pos = StringFind(json, search);
   if(key_pos == -1) return;

   int array_start = StringFind(json, "[", key_pos);
   if(array_start == -1) return;

   int array_end = StringFind(json, "]", array_start);
   if(array_end == -1) return;

   string array_content = StringSubstr(json, array_start + 1, array_end - array_start - 1);
   StringTrimLeft(array_content);
   StringTrimRight(array_content);

   // Check for null or empty
   if(array_content == "" || StringFind(array_content, "null") == 0) return;

   // Split by comma
   string items[];
   int count = StringSplit(array_content, ',', items);

   for(int i = 0; i < count; i++)
   {
      string item = items[i];
      StringTrimLeft(item);
      StringTrimRight(item);

      if(item != "")
      {
         int idx = ArraySize(output);
         ArrayResize(output, idx + 1);
         output[idx] = (int)StringToInteger(item);
      }
   }
}
