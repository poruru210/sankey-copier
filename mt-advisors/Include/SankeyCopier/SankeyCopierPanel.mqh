//+------------------------------------------------------------------+
//|                                         SankeyCopierPanel.mqh    |
//|                          Sankey Copier Configuration Panel Display|
//|                                                                  |
//| Purpose: Provides unified panel display functions for Slave EA  |
//|          Shows current configuration status on the chart        |
//|          MT4/MT5 compatible implementation                      |
//+------------------------------------------------------------------+
#property copyright "Sankey Copier"
#property strict

// Platform detection
#ifndef IS_MT4
#ifndef IS_MT5
   #ifdef __MQL5__
      #define IS_MT5
   #else
      #define IS_MT4
   #endif
#endif
#endif

// Panel configuration
#define PANEL_PREFIX "SankeyCopierPanel_"
#define PANEL_FONT "Arial"
#define PANEL_FONT_SIZE 8
#define PANEL_COLOR_BG C'40,40,50'        // Dark semi-transparent background
#define PANEL_COLOR_BORDER clrDimGray      // Border color for better visibility
#define PANEL_COLOR_TITLE clrWhite
#define PANEL_COLOR_LABEL clrLightGray
#define PANEL_COLOR_VALUE clrWhite
#define PANEL_COLOR_ENABLED clrLime
#define PANEL_COLOR_DISABLED clrRed

//+------------------------------------------------------------------+
//| Create text label object (MT4/MT5 compatible)                    |
//+------------------------------------------------------------------+
void CreatePanelLabel(string name, int x, int y, string text, color clr, int font_size = PANEL_FONT_SIZE)
{
   #ifdef IS_MT5
      ObjectCreate(0, name, OBJ_LABEL, 0, 0, 0);
      ObjectSetInteger(0, name, OBJPROP_XDISTANCE, x);
      ObjectSetInteger(0, name, OBJPROP_YDISTANCE, y);
      ObjectSetInteger(0, name, OBJPROP_CORNER, CORNER_RIGHT_UPPER);
      ObjectSetInteger(0, name, OBJPROP_ANCHOR, ANCHOR_RIGHT_UPPER);
      ObjectSetString(0, name, OBJPROP_TEXT, text);
      ObjectSetString(0, name, OBJPROP_FONT, PANEL_FONT);
      ObjectSetInteger(0, name, OBJPROP_FONTSIZE, font_size);
      ObjectSetInteger(0, name, OBJPROP_COLOR, clr);
      ObjectSetInteger(0, name, OBJPROP_BACK, false);
      ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
      ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
   #else
      ObjectCreate(name, OBJ_LABEL, 0, 0, 0);
      ObjectSet(name, OBJPROP_XDISTANCE, x);
      ObjectSet(name, OBJPROP_YDISTANCE, y);
      ObjectSet(name, OBJPROP_CORNER, CORNER_RIGHT_UPPER);
      ObjectSet(name, OBJPROP_ANCHOR, ANCHOR_RIGHT_UPPER);
      ObjectSetText(name, text, font_size, PANEL_FONT, clr);
      ObjectSet(name, OBJPROP_BACK, false);
      ObjectSet(name, OBJPROP_SELECTABLE, false);
      ObjectSet(name, OBJPROP_HIDDEN, true);
   #endif
}

//+------------------------------------------------------------------+
//| Update text label (MT4/MT5 compatible)                           |
//+------------------------------------------------------------------+
void UpdatePanelLabel(string name, string text, color clr = -1)
{
   #ifdef IS_MT5
      ObjectSetString(0, name, OBJPROP_TEXT, text);
      if(clr != -1)
         ObjectSetInteger(0, name, OBJPROP_COLOR, clr);
   #else
      int font_size = ObjectGet(name, OBJPROP_FONTSIZE);
      if(font_size == 0) font_size = PANEL_FONT_SIZE;
      if(clr == -1) clr = ObjectGet(name, OBJPROP_COLOR);
      ObjectSetText(name, text, font_size, PANEL_FONT, clr);
   #endif
}

//+------------------------------------------------------------------+
//| Create rectangle label (background) - MT4/MT5 compatible         |
//+------------------------------------------------------------------+
void CreatePanelBackground(string name, int x, int y, int width, int height, color bg_color)
{
   #ifdef IS_MT5
      ObjectCreate(0, name, OBJ_RECTANGLE_LABEL, 0, 0, 0);
      ObjectSetInteger(0, name, OBJPROP_XDISTANCE, x);
      ObjectSetInteger(0, name, OBJPROP_YDISTANCE, y);
      ObjectSetInteger(0, name, OBJPROP_XSIZE, width);
      ObjectSetInteger(0, name, OBJPROP_YSIZE, height);
      ObjectSetInteger(0, name, OBJPROP_CORNER, CORNER_RIGHT_UPPER);
      ObjectSetInteger(0, name, OBJPROP_BGCOLOR, bg_color);
      ObjectSetInteger(0, name, OBJPROP_BORDER_TYPE, BORDER_FLAT);
      ObjectSetInteger(0, name, OBJPROP_COLOR, PANEL_COLOR_BORDER);
      ObjectSetInteger(0, name, OBJPROP_WIDTH, 1);
      ObjectSetInteger(0, name, OBJPROP_BACK, false);  // Draw in foreground for better visibility
      ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
      ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
   #else
      ObjectCreate(name, OBJ_RECTANGLE_LABEL, 0, 0, 0);
      ObjectSet(name, OBJPROP_XDISTANCE, x);
      ObjectSet(name, OBJPROP_YDISTANCE, y);
      ObjectSet(name, OBJPROP_XSIZE, width);
      ObjectSet(name, OBJPROP_YSIZE, height);
      ObjectSet(name, OBJPROP_CORNER, CORNER_RIGHT_UPPER);
      ObjectSet(name, OBJPROP_BGCOLOR, bg_color);
      ObjectSet(name, OBJPROP_BORDER_TYPE, BORDER_FLAT);
      ObjectSet(name, OBJPROP_COLOR, PANEL_COLOR_BORDER);
      ObjectSet(name, OBJPROP_WIDTH, 1);
      ObjectSet(name, OBJPROP_BACK, false);  // Draw in foreground for better visibility
      ObjectSet(name, OBJPROP_SELECTABLE, false);
      ObjectSet(name, OBJPROP_HIDDEN, true);
   #endif
}

//+------------------------------------------------------------------+
//| Initialize configuration panel                                    |
//| Parameters:                                                       |
//|   x_offset - X offset from right edge (default: 10)             |
//|   y_offset - Y offset from top edge (default: 20)               |
//+------------------------------------------------------------------+
void InitializePanel(int x_offset = 10, int y_offset = 20)
{
   int panel_width = 200;
   int panel_height = 120;
   int line_height = 15;
   int x = x_offset;
   int y = y_offset;

   // Create background
   CreatePanelBackground(PANEL_PREFIX + "BG", x, y, panel_width, panel_height, PANEL_COLOR_BG);

   // Create title (centered)
   CreatePanelLabel(PANEL_PREFIX + "Title", x + 100, y + 3, "Sankey Copier - Slave", PANEL_COLOR_TITLE, 9);

   // Create labels (left column) - ANCHOR_RIGHT_UPPER means larger x = more left
   CreatePanelLabel(PANEL_PREFIX + "StatusLabel", x + 190, y + 20, "Status:", PANEL_COLOR_LABEL);
   CreatePanelLabel(PANEL_PREFIX + "MasterLabel", x + 190, y + 35, "Master:", PANEL_COLOR_LABEL);
   CreatePanelLabel(PANEL_PREFIX + "LotLabel", x + 190, y + 50, "Lot Mult:", PANEL_COLOR_LABEL);
   CreatePanelLabel(PANEL_PREFIX + "ReverseLabel", x + 190, y + 65, "Reverse:", PANEL_COLOR_LABEL);
   CreatePanelLabel(PANEL_PREFIX + "VersionLabel", x + 190, y + 80, "Config Ver:", PANEL_COLOR_LABEL);
   CreatePanelLabel(PANEL_PREFIX + "SymbolsLabel", x + 190, y + 95, "Symbols:", PANEL_COLOR_LABEL);

   // Create value labels (right column) - will be updated dynamically
   CreatePanelLabel(PANEL_PREFIX + "Status", x + 15, y + 20, "DISABLED", PANEL_COLOR_DISABLED);
   CreatePanelLabel(PANEL_PREFIX + "Master", x + 15, y + 35, "N/A", PANEL_COLOR_VALUE);
   CreatePanelLabel(PANEL_PREFIX + "Lot", x + 15, y + 50, "1.00x", PANEL_COLOR_VALUE);
   CreatePanelLabel(PANEL_PREFIX + "Reverse", x + 15, y + 65, "OFF", PANEL_COLOR_VALUE);
   CreatePanelLabel(PANEL_PREFIX + "Version", x + 15, y + 80, "0", PANEL_COLOR_VALUE);
   CreatePanelLabel(PANEL_PREFIX + "Symbols", x + 15, y + 95, "0", PANEL_COLOR_VALUE);

   Print("Configuration panel initialized");
}

//+------------------------------------------------------------------+
//| Update panel with current configuration                          |
//| Parameters:                                                       |
//|   enabled        - Copy enabled status                           |
//|   master_account - Master account ID                             |
//|   lot_mult       - Lot multiplier                                |
//|   reverse        - Reverse trade flag                            |
//|   config_ver     - Configuration version                         |
//|   symbol_count   - Number of symbol mappings                     |
//+------------------------------------------------------------------+
void UpdatePanel(bool enabled, string master_account, double lot_mult, bool reverse, int config_ver, int symbol_count)
{
   // Update status
   if(enabled)
      UpdatePanelLabel(PANEL_PREFIX + "Status", "ENABLED", PANEL_COLOR_ENABLED);
   else
      UpdatePanelLabel(PANEL_PREFIX + "Status", "DISABLED", PANEL_COLOR_DISABLED);

   // Update master account (truncate if too long)
   string master_display = master_account;
   if(StringLen(master_display) > 15)
      master_display = StringSubstr(master_display, 0, 12) + "...";
   if(master_display == "")
      master_display = "N/A";
   UpdatePanelLabel(PANEL_PREFIX + "Master", master_display, PANEL_COLOR_VALUE);

   // Update lot multiplier
   string lot_str = DoubleToString(lot_mult, 2) + "x";
   UpdatePanelLabel(PANEL_PREFIX + "Lot", lot_str, PANEL_COLOR_VALUE);

   // Update reverse flag
   UpdatePanelLabel(PANEL_PREFIX + "Reverse", reverse ? "ON" : "OFF", PANEL_COLOR_VALUE);

   // Update config version
   UpdatePanelLabel(PANEL_PREFIX + "Version", IntegerToString(config_ver), PANEL_COLOR_VALUE);

   // Update symbol count
   UpdatePanelLabel(PANEL_PREFIX + "Symbols", IntegerToString(symbol_count), PANEL_COLOR_VALUE);
}

//+------------------------------------------------------------------+
//| Delete all panel objects                                          |
//+------------------------------------------------------------------+
void DeletePanel()
{
   #ifdef IS_MT5
      ObjectDelete(0, PANEL_PREFIX + "BG");
      ObjectDelete(0, PANEL_PREFIX + "Title");
      ObjectDelete(0, PANEL_PREFIX + "StatusLabel");
      ObjectDelete(0, PANEL_PREFIX + "MasterLabel");
      ObjectDelete(0, PANEL_PREFIX + "LotLabel");
      ObjectDelete(0, PANEL_PREFIX + "ReverseLabel");
      ObjectDelete(0, PANEL_PREFIX + "VersionLabel");
      ObjectDelete(0, PANEL_PREFIX + "SymbolsLabel");
      ObjectDelete(0, PANEL_PREFIX + "Status");
      ObjectDelete(0, PANEL_PREFIX + "Master");
      ObjectDelete(0, PANEL_PREFIX + "Lot");
      ObjectDelete(0, PANEL_PREFIX + "Reverse");
      ObjectDelete(0, PANEL_PREFIX + "Version");
      ObjectDelete(0, PANEL_PREFIX + "Symbols");
   #else
      ObjectDelete(PANEL_PREFIX + "BG");
      ObjectDelete(PANEL_PREFIX + "Title");
      ObjectDelete(PANEL_PREFIX + "StatusLabel");
      ObjectDelete(PANEL_PREFIX + "MasterLabel");
      ObjectDelete(PANEL_PREFIX + "LotLabel");
      ObjectDelete(PANEL_PREFIX + "ReverseLabel");
      ObjectDelete(PANEL_PREFIX + "VersionLabel");
      ObjectDelete(PANEL_PREFIX + "SymbolsLabel");
      ObjectDelete(PANEL_PREFIX + "Status");
      ObjectDelete(PANEL_PREFIX + "Master");
      ObjectDelete(PANEL_PREFIX + "Lot");
      ObjectDelete(PANEL_PREFIX + "Reverse");
      ObjectDelete(PANEL_PREFIX + "Version");
      ObjectDelete(PANEL_PREFIX + "Symbols");
   #endif

   Print("Configuration panel deleted");
}
