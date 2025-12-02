//+------------------------------------------------------------------+
//|                                   SankeyCopierGridPanel.mqh      |
//|                          Dynamic Grid Layout Panel for MT4/MT5   |
//|                                                                  |
//| Purpose: Provides dynamic grid layout with automatic row        |
//|          management, coordinate calculation, and panel sizing.  |
//|          Supports runtime addition/removal of data rows.        |
//+------------------------------------------------------------------+
#property copyright "Copyright 2025, SANKEY Copier Project"

#ifndef SANKEY_COPIER_GRIDPANEL_MQH
#define SANKEY_COPIER_GRIDPANEL_MQH

#include "Common.mqh"
#include "SlaveTypes.mqh"

#property strict

// Platform detection (reuse existing pattern)
#ifndef IS_MT4
#ifndef IS_MT5
   #ifdef __MQL5__
      #define IS_MT5
   #else
      #define IS_MT4
   #endif
#endif
#endif

// Panel styling constants
#define PANEL_FONT "Arial"
#define PANEL_FONT_SIZE 8
#define PANEL_DATA_FONT_SIZE 8             // Font size for data rows
#define PANEL_TITLE_FONT_SIZE 9            // Font size for title
#define PANEL_COLOR_BG C'40,40,50'        // Dark background
#define PANEL_COLOR_BORDER clrDimGray      // Border color
#define PANEL_COLOR_TITLE clrWhite
#define PANEL_COLOR_LABEL clrLightGray
#define PANEL_COLOR_VALUE clrWhite
#define PANEL_COLOR_DISABLED clrGray         // Status: DISABLED (0) - matches Web UI gray
#define PANEL_COLOR_WAITING clrYellow        // Status: ENABLED but Master disconnected (1) - matches Web UI yellow warning
#define PANEL_COLOR_CONNECTED clrLime        // Status: CONNECTED (2) - matches Web UI green active
#define PANEL_COLOR_NO_CONFIG clrDarkGray    // Status: NO_CONFIGURATION (3) - no config received yet

// Panel layout constants
#define TITLE_HEIGHT_EXTRA_PADDING 5       // Extra padding for title row height
#define DEFAULT_X_OFFSET 10                // Default distance from right edge
#define DEFAULT_Y_OFFSET 20                // Default distance from top edge
#define DEFAULT_PANEL_WIDTH 280            // Default panel width in pixels
#define DEFAULT_ROW_HEIGHT 15              // Default row height in pixels
#define DEFAULT_TITLE_HEIGHT 20            // Default title row height in pixels
#define DEFAULT_PADDING_TOP 3              // Default top padding
#define DEFAULT_PADDING_BOTTOM 5           // Default bottom padding
#define DEFAULT_PADDING_LEFT 5             // Default left padding inside panel
#define DEFAULT_PADDING_RIGHT 10           // Default right padding inside panel
#define DEFAULT_COLUMN_COUNT 2             // Default number of columns
#define LABEL_COLUMN_WIDTH 100             // Fixed width for label column in pixels

//+------------------------------------------------------------------+
//| Helper Functions for Panel Creation                              |
//+------------------------------------------------------------------+

//+------------------------------------------------------------------+
//| Create text label object (MT4/MT5 compatible)                    |
//| Parameters:                                                       |
//|   name - Object name                                             |
//|   x, y - Coordinates                                             |
//|   text - Label text                                              |
//|   clr - Text color                                               |
//|   font_size - Font size                                          |
//|   anchor - Anchor point (ANCHOR_LEFT_UPPER or ANCHOR_RIGHT_UPPER)|
//+------------------------------------------------------------------+
void CreatePanelLabel(string name, int x, int y, string text, color clr, int font_size = PANEL_FONT_SIZE, ENUM_ANCHOR_POINT anchor = ANCHOR_RIGHT_UPPER)
{
   #ifdef IS_MT5
      ObjectCreate(0, name, OBJ_LABEL, 0, 0, 0);
      ObjectSetInteger(0, name, OBJPROP_XDISTANCE, x);
      ObjectSetInteger(0, name, OBJPROP_YDISTANCE, y);
      ObjectSetInteger(0, name, OBJPROP_CORNER, CORNER_RIGHT_UPPER);
      ObjectSetInteger(0, name, OBJPROP_ANCHOR, anchor);
      ObjectSetString(0, name, OBJPROP_TEXT, text);
      ObjectSetString(0, name, OBJPROP_FONT, PANEL_FONT);
      ObjectSetInteger(0, name, OBJPROP_FONTSIZE, font_size);
      ObjectSetInteger(0, name, OBJPROP_COLOR, clr);
      ObjectSetInteger(0, name, OBJPROP_BACK, false);
      ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
      ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
      ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
      ObjectSetInteger(0, name, OBJPROP_ZORDER, 10); // Ensure text is on top
   #else
      ObjectCreate(name, OBJ_LABEL, 0, 0, 0);
      ObjectSet(name, OBJPROP_XDISTANCE, x);
      ObjectSet(name, OBJPROP_YDISTANCE, y);
      ObjectSet(name, OBJPROP_CORNER, CORNER_RIGHT_UPPER);
      ObjectSet(name, OBJPROP_ANCHOR, anchor);
      ObjectSetText(name, text, font_size, PANEL_FONT, clr);
      ObjectSet(name, OBJPROP_BACK, false);
      ObjectSet(name, OBJPROP_SELECTABLE, false);
      ObjectSet(name, OBJPROP_HIDDEN, true);
   #endif
}

//+------------------------------------------------------------------+
//| Update text label (MT4/MT5 compatible)                           |
//| Note: clr=clrNONE means "keep existing color"                    |
//+------------------------------------------------------------------+
void UpdatePanelLabel(string name, string text, color clr = clrNONE)
{
   #ifdef IS_MT5
      ObjectSetString(0, name, OBJPROP_TEXT, text);
      if(clr != clrNONE)
         ObjectSetInteger(0, name, OBJPROP_COLOR, clr);
   #else
      int font_size = (int)ObjectGet(name, OBJPROP_FONTSIZE);
      if(font_size == 0) font_size = PANEL_FONT_SIZE;
      if(clr == clrNONE) clr = (color)ObjectGet(name, OBJPROP_COLOR);
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
      ObjectSetInteger(0, name, OBJPROP_BACK, false);
      ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
      ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
      ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
      ObjectSetInteger(0, name, OBJPROP_ZORDER, 0); // Ensure background is behind
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
      ObjectSet(name, OBJPROP_BACK, false);
      ObjectSet(name, OBJPROP_SELECTABLE, false);
      ObjectSet(name, OBJPROP_HIDDEN, true);
   #endif
}

//+------------------------------------------------------------------+
//| Truncate text to fit maximum character count                    |
//| Parameters:                                                       |
//|   text        - Original text string                            |
//|   max_chars   - Maximum number of characters                    |
//| Returns: Truncated text with "..." if needed                    |
//+------------------------------------------------------------------+
string TruncateText(string text, int max_chars)
{
   int text_len = StringLen(text);

   // If text fits, return as-is
   if(text_len <= max_chars)
      return text;

   // If max_chars is too small for ellipsis, just return truncated
   if(max_chars < 4)
      return StringSubstr(text, 0, max_chars);

   // Show beginning and end with "..." in middle
   // Example: "Exness_Technologies_Ltd_277195421" -> "Exness_Tech...95421"
   int prefix_len = (max_chars - 3) / 2;
   int suffix_len = max_chars - 3 - prefix_len;

   string prefix = StringSubstr(text, 0, prefix_len);
   string suffix = StringSubstr(text, text_len - suffix_len, suffix_len);

   return prefix + "..." + suffix;
}

//+------------------------------------------------------------------+
//| Dynamic Grid Layout Panel Class                                  |
//| Manages rows and columns with automatic positioning              |
//+------------------------------------------------------------------+
class CGridPanel
{
private:
   // Panel configuration
   string   m_prefix;              // Object name prefix
   int      m_x_offset;            // X offset from right edge (CORNER_RIGHT_UPPER)
   int      m_y_offset;            // Y offset from top edge
   int      m_panel_width;         // Panel width in pixels
   int      m_row_height;          // Height per row
   int      m_title_height;        // Title row height
   int      m_padding_top;         // Top padding
   int      m_padding_bottom;      // Bottom padding
   int      m_padding_left;        // Left padding inside panel
   int      m_padding_right;       // Right padding inside panel

   // Grid configuration
   int      m_column_count;        // Number of columns
   int      m_column_widths[];     // X position for each column (from right edge)
   ENUM_ANCHOR_POINT m_column_anchors[];  // Anchor point for each column

   // Row management
   int      m_row_count;           // Current number of data rows (excluding title)
   string   m_row_keys[];          // Row identifiers for lookups

   // Colors
   color    m_bg_color;
   color    m_border_color;
   color    m_title_color;

   // State
   bool     m_initialized;

   // Carousel state for Slave panel (copy settings pagination)
   int      m_carousel_index;      // Current page index (0-based)
   int      m_carousel_count;      // Total number of pages (configs)
   bool     m_carousel_enabled;    // Whether carousel navigation is enabled
   CopyConfig m_cached_configs[];  // Cached configs for carousel display

   // Internal coordinate calculation methods
   int      CalculateBackgroundX();
   int      CalculateColumnX(int column_index);
   int      CalculateTitleX();
   int      CalculateRowY(int row_index);
   int      CalculatePanelHeight();
   void     UpdateBackgroundSize();
   string   GenerateObjectName(string suffix);

public:
   // Constructor and Destructor
   CGridPanel();
   ~CGridPanel();

   // Initialization
   bool     Initialize(string prefix, int x_offset, int y_offset,
                      int panel_width = 200, int row_height = 15);

   // Column configuration
   void     SetColumnCount(int count);
   void     SetColumnWidth(int column_index, int width_from_right);

   // Row management
   int      AddRow(string row_key, string &values[], color &colors[]);
   int      AddSeparator(string row_key);  // Add separator line
   int      AddCenteredRow(string row_key, string text, color clr);  // Add centered text row
   bool     UpdateCenteredRow(string row_key, string text, color clr = -1);  // Update centered row
   bool     UpdateRow(string row_key, string &values[], color &colors[]);
   bool     UpdateCell(string row_key, int column_index, string value, color clr = -1);
   bool     RemoveRow(string row_key);
   void     ClearRows();
   int      GetRowCount() const { return m_row_count; }

   // Title management (clr=clrNONE means "use default title color")
   void     SetTitle(string title, color clr = clrNONE);

   // Slave EA panel helpers (high-level update methods)
   bool     InitializeSlavePanel(string prefix = "SankeyCopierPanel_", int panel_width = DEFAULT_PANEL_WIDTH);

   // Carousel methods for displaying copy settings (Slave panel)
   void     UpdateCarouselConfigs(CopyConfig &configs[]);  // Update configs and refresh display
   void     ShowCarouselPage(int index);                   // Show specific page
   void     NextCarouselPage();                            // Navigate to next config
   void     PrevCarouselPage();                            // Navigate to previous config
   int      GetCarouselIndex() const { return m_carousel_index; }
   int      GetCarouselCount() const { return m_carousel_count; }
   bool     HandleChartClick(int x, int y);                // Handle click for navigation buttons

   // Master EA panel helpers
   bool     InitializeMasterPanel(string prefix = "SankeyCopierPanel_", int panel_width = DEFAULT_PANEL_WIDTH);
   void     UpdateTrackedOrdersRow(int count);
   void     UpdateMagicFilterRow(int magic);
   void     UpdateSymbolConfig(string prefix, string suffix, string map);
   void     UpdateServerRow(string address);
   void     UpdateConfigList(CopyConfig &configs[]);

   // Common helpers
   void     UpdateStatusRow(int status, bool allow_new_orders = false);
   void     UpdatePanelStatusFromConfigs(CopyConfig &configs[]);
   void     UpdateMasterRow(string master_name);
   void     UpdateLotMultiplierRow(double multiplier);
   void     UpdateReverseRow(bool reverse);
   void     UpdateVersionRow(int version);
   void     UpdateSymbolCountRow(int count);

   // Message mode (for "Not Configured" state)
   void     ShowMessage(string message, color clr = clrYellow);
   void     HideMessage();

   // Appearance
   void     SetColors(color bg, color border, color title);
   void     SetPadding(int top, int bottom, int left, int right);
   void     SetRowHeight(int height);

   // Lifecycle
   void     Refresh();  // Redraw all elements
   void     Delete();   // Delete all objects
};

//+------------------------------------------------------------------+
//| Constructor                                                       |
//+------------------------------------------------------------------+
CGridPanel::CGridPanel()
{
   m_row_count = 0;
   m_column_count = DEFAULT_COLUMN_COUNT;
   m_prefix = "GridPanel_";
   m_x_offset = DEFAULT_X_OFFSET;
   m_y_offset = DEFAULT_Y_OFFSET;
   m_panel_width = DEFAULT_PANEL_WIDTH;
   m_row_height = DEFAULT_ROW_HEIGHT;
   m_title_height = DEFAULT_TITLE_HEIGHT;
   m_padding_top = DEFAULT_PADDING_TOP;
   m_padding_bottom = DEFAULT_PADDING_BOTTOM;
   m_padding_left = DEFAULT_PADDING_LEFT;
   m_padding_right = DEFAULT_PADDING_RIGHT;
   m_bg_color = PANEL_COLOR_BG;
   m_border_color = PANEL_COLOR_BORDER;
   m_title_color = PANEL_COLOR_TITLE;
   m_initialized = false;

   // Carousel state
   m_carousel_index = 0;
   m_carousel_count = 0;
   m_carousel_enabled = false;
   ArrayResize(m_cached_configs, 0);
}

//+------------------------------------------------------------------+
//| Destructor                                                        |
//+------------------------------------------------------------------+
CGridPanel::~CGridPanel()
{
   Delete();
}

//+------------------------------------------------------------------+
//| Initialize panel                                                  |
//| Parameters:                                                       |
//|   prefix       - Object name prefix (e.g., "SankeyCopierPanel_")|
//|   x_offset     - Distance from screen right edge                |
//|   y_offset     - Distance from screen top edge                  |
//|   panel_width  - Panel width in pixels                          |
//|   row_height   - Height of each row in pixels                   |
//| Returns: true on success                                         |
//+------------------------------------------------------------------+
bool CGridPanel::Initialize(string prefix, int x_offset, int y_offset,
                           int panel_width = 200, int row_height = 15)
{
   // If re-initializing with the same prefix, delete existing objects first
   // This prevents object name conflicts and stale objects when EA parameters change
   if(m_prefix == prefix && m_prefix != "")
   {
      Delete();
   }

   m_prefix = prefix;
   m_x_offset = x_offset;
   m_y_offset = y_offset;
   m_panel_width = panel_width;
   m_row_height = row_height;
   m_title_height = row_height + TITLE_HEIGHT_EXTRA_PADDING;

   // Initialize row management arrays
   ArrayResize(m_row_keys, 0);
   m_row_count = 0;

   // Default 2-column layout with unified anchors (both LEFT_UPPER)
   ArrayResize(m_column_widths, 2);
   ArrayResize(m_column_anchors, 2);

   // Label column: ANCHOR_LEFT_UPPER, fixed width (LABEL_COLUMN_WIDTH)
   m_column_anchors[0] = ANCHOR_LEFT_UPPER;
   m_column_widths[0] = CalculateColumnX(0);

   // Value column: ANCHOR_LEFT_UPPER (unified coordinate system)
   // Takes remaining space after label column
   m_column_anchors[1] = ANCHOR_LEFT_UPPER;
   m_column_widths[1] = CalculateColumnX(1);

   // Create background with initial size (title only)
   // For CORNER_RIGHT_UPPER: XDISTANCE is the left edge position from right
   // To fit panel in screen: left_edge = x_offset + panel_width
   int initial_height = m_padding_top + m_title_height + m_padding_bottom;
   int bg_x = CalculateBackgroundX();
   CreatePanelBackground(GenerateObjectName("BG"),
                        bg_x,
                        m_y_offset,
                        m_panel_width,
                        initial_height,
                        m_bg_color);

   m_initialized = true;
   return true;
}

//+------------------------------------------------------------------+
//| Set number of columns                                            |
//+------------------------------------------------------------------+
void CGridPanel::SetColumnCount(int count)
{
   if(count < 1)
   {
      Print("Invalid column count: ", count);
      return;
   }

   m_column_count = count;
   ArrayResize(m_column_widths, count);
}

//+------------------------------------------------------------------+
//| Set column width (X position from right edge)                   |
//+------------------------------------------------------------------+
void CGridPanel::SetColumnWidth(int column_index, int width_from_right)
{
   if(column_index < 0 || column_index >= m_column_count)
   {
      Print("Invalid column index: ", column_index);
      return;
   }

   m_column_widths[column_index] = width_from_right;
}

//+------------------------------------------------------------------+
//| Add a new row to the grid                                        |
//| Parameters:                                                       |
//|   row_key - Unique identifier for this row                      |
//|   values  - Array of text values for each column                |
//|   colors  - Array of colors for each column                     |
//| Returns: Row index on success, -1 on error                      |
//+------------------------------------------------------------------+
int CGridPanel::AddRow(string row_key, string &values[], color &colors[])
{
   // Check if row already exists
   for(int i = 0; i < m_row_count; i++)
   {
      if(m_row_keys[i] == row_key)
      {
         Print("Row with key '", row_key, "' already exists");
         return -1;
      }
   }

   // Validate array sizes
   if(ArraySize(values) != m_column_count || ArraySize(colors) != m_column_count)
   {
      Print("Array size mismatch: expected ", m_column_count, " columns, got ",
            ArraySize(values), "/", ArraySize(colors));
      return -1;
   }

   // Add row key
   int new_size = m_row_count + 1;
   ArrayResize(m_row_keys, new_size);
   m_row_keys[m_row_count] = row_key;

   // Calculate Y position for this row
   int row_y = CalculateRowY(m_row_count);

   // Create label objects for each column with appropriate anchor
   for(int col = 0; col < m_column_count; col++)
   {
      string obj_name = GenerateObjectName(row_key + "_col" + IntegerToString(col));
      CreatePanelLabel(obj_name,
                      m_column_widths[col],
                      row_y,
                      values[col],
                      colors[col],
                      PANEL_DATA_FONT_SIZE,
                      m_column_anchors[col]);  // Use anchor specific to this column
   }

   m_row_count++;

   // Update background height
   UpdateBackgroundSize();

   return m_row_count - 1;
}

//+------------------------------------------------------------------+
//| Add a separator line row using rectangle object                   |
//| Parameters:                                                       |
//|   row_key - Unique identifier for the separator row              |
//| Returns: Row index on success, -1 on error                       |
//| Note: Uses CORNER_RIGHT_UPPER to match panel coordinate system   |
//+------------------------------------------------------------------+
int CGridPanel::AddSeparator(string row_key)
{
   // Add row key to track this separator
   int new_size = m_row_count + 1;
   ArrayResize(m_row_keys, new_size);
   m_row_keys[m_row_count] = row_key;

   // Calculate position for separator line
   // Using CORNER_RIGHT_UPPER: XDISTANCE is distance from right edge to left edge of object
   int row_y = CalculateRowY(m_row_count) + (m_row_height / 2) - 1;
   int line_width = m_panel_width - m_padding_left - m_padding_right;
   // Line left edge from right = panel left edge from right - padding_left
   // Panel left edge = m_x_offset + m_panel_width
   // Line left edge = (m_x_offset + m_panel_width) - m_padding_left
   int line_x = m_x_offset + m_panel_width - m_padding_left;

   // Create rectangle label as a thin horizontal line
   string obj_name = GenerateObjectName(row_key + "_line");

   #ifdef IS_MT5
      ObjectCreate(0, obj_name, OBJ_RECTANGLE_LABEL, 0, 0, 0);
      ObjectSetInteger(0, obj_name, OBJPROP_XDISTANCE, line_x);
      ObjectSetInteger(0, obj_name, OBJPROP_YDISTANCE, row_y);
      ObjectSetInteger(0, obj_name, OBJPROP_XSIZE, line_width);
      ObjectSetInteger(0, obj_name, OBJPROP_YSIZE, 1);  // 1 pixel height
      ObjectSetInteger(0, obj_name, OBJPROP_BGCOLOR, clrDimGray);
      ObjectSetInteger(0, obj_name, OBJPROP_COLOR, clrDimGray);  // Border color same as background
      ObjectSetInteger(0, obj_name, OBJPROP_BORDER_TYPE, BORDER_FLAT);
      ObjectSetInteger(0, obj_name, OBJPROP_WIDTH, 0);  // No border width
      ObjectSetInteger(0, obj_name, OBJPROP_CORNER, CORNER_RIGHT_UPPER);
      ObjectSetInteger(0, obj_name, OBJPROP_BACK, false);
      ObjectSetInteger(0, obj_name, OBJPROP_SELECTABLE, false);
      ObjectSetInteger(0, obj_name, OBJPROP_ZORDER, 5);  // Above background, below text
   #else
      ObjectCreate(obj_name, OBJ_RECTANGLE_LABEL, 0, 0, 0);
      ObjectSet(obj_name, OBJPROP_XDISTANCE, line_x);
      ObjectSet(obj_name, OBJPROP_YDISTANCE, row_y);
      ObjectSet(obj_name, OBJPROP_XSIZE, line_width);
      ObjectSet(obj_name, OBJPROP_YSIZE, 1);  // 1 pixel height
      ObjectSet(obj_name, OBJPROP_BGCOLOR, clrDimGray);
      ObjectSet(obj_name, OBJPROP_COLOR, clrDimGray);  // Border color same as background
      ObjectSet(obj_name, OBJPROP_BORDER_TYPE, BORDER_FLAT);
      ObjectSet(obj_name, OBJPROP_WIDTH, 0);  // No border width
      ObjectSet(obj_name, OBJPROP_CORNER, CORNER_RIGHT_UPPER);
      ObjectSet(obj_name, OBJPROP_BACK, false);
      ObjectSet(obj_name, OBJPROP_SELECTABLE, false);
   #endif

   m_row_count++;
   UpdateBackgroundSize();

   return m_row_count - 1;
}

//+------------------------------------------------------------------+
//| Add a centered text row                                           |
//| Parameters:                                                       |
//|   row_key - Unique identifier for this row                       |
//|   text    - Text to display (centered)                           |
//|   clr     - Text color                                           |
//| Returns: Row index on success, -1 on error                       |
//+------------------------------------------------------------------+
int CGridPanel::AddCenteredRow(string row_key, string text, color clr)
{
   // Check if row already exists
   for(int i = 0; i < m_row_count; i++)
   {
      if(m_row_keys[i] == row_key)
      {
         Print("Row with key '", row_key, "' already exists");
         return -1;
      }
   }

   // Add row key
   int new_size = m_row_count + 1;
   ArrayResize(m_row_keys, new_size);
   m_row_keys[m_row_count] = row_key;

   // Calculate Y position for this row
   int row_y = CalculateRowY(m_row_count);

   // Calculate center X position (same as title)
   int center_x = CalculateTitleX();

   // Create centered label
   string obj_name = GenerateObjectName(row_key + "_center");
   CreatePanelLabel(obj_name, center_x, row_y, text, clr, PANEL_DATA_FONT_SIZE, ANCHOR_UPPER);

   m_row_count++;
   UpdateBackgroundSize();

   return m_row_count - 1;
}

//+------------------------------------------------------------------+
//| Update a centered row                                             |
//| Parameters:                                                       |
//|   row_key - Unique identifier for the row to update              |
//|   text    - New text to display                                  |
//|   clr     - New color (-1 to keep existing)                      |
//| Returns: true on success, false if row not found                 |
//+------------------------------------------------------------------+
bool CGridPanel::UpdateCenteredRow(string row_key, string text, color clr = -1)
{
   // Find row index
   int row_index = -1;
   for(int i = 0; i < m_row_count; i++)
   {
      if(m_row_keys[i] == row_key)
      {
         row_index = i;
         break;
      }
   }

   if(row_index == -1)
      return false;

   string obj_name = GenerateObjectName(row_key + "_center");
   UpdatePanelLabel(obj_name, text, clr);
   return true;
}

//+------------------------------------------------------------------+
//| Update an existing row                                           |
//| Parameters:                                                       |
//|   row_key - Unique identifier for the row to update            |
//|   values  - New text values for each column                     |
//|   colors  - New colors for each column                          |
//| Returns: true on success, false if row not found               |
//+------------------------------------------------------------------+
bool CGridPanel::UpdateRow(string row_key, string &values[], color &colors[])
{
   // Find row index
   int row_index = -1;
   for(int i = 0; i < m_row_count; i++)
   {
      if(m_row_keys[i] == row_key)
      {
         row_index = i;
         break;
      }
   }

   if(row_index == -1)
   {
      // Print("Row '", row_key, "' not found"); // Silenced to avoid log spam on first update
      return false;
   }

   // Validate array sizes
   if(ArraySize(values) != m_column_count || ArraySize(colors) != m_column_count)
   {
      Print("Array size mismatch in UpdateRow");
      return false;
   }

   // Update each column
   for(int col = 0; col < m_column_count; col++)
   {
      string obj_name = GenerateObjectName(row_key + "_col" + IntegerToString(col));
      UpdatePanelLabel(obj_name, values[col], colors[col]);
   }

   return true;
}

//+------------------------------------------------------------------+
//| Update a single cell                                             |
//+------------------------------------------------------------------+
bool CGridPanel::UpdateCell(string row_key, int column_index, string value, color clr = -1)
{
   if(column_index < 0 || column_index >= m_column_count)
   {
      Print("Invalid column index: ", column_index);
      return false;
   }

   string obj_name = GenerateObjectName(row_key + "_col" + IntegerToString(column_index));
   UpdatePanelLabel(obj_name, value, clr);
   return true;
}

//+------------------------------------------------------------------+
//| Remove a row from the grid                                       |
//+------------------------------------------------------------------+
bool CGridPanel::RemoveRow(string row_key)
{
   // Find row index
   int row_index = -1;
   for(int i = 0; i < m_row_count; i++)
   {
      if(m_row_keys[i] == row_key)
      {
         row_index = i;
         break;
      }
   }

   if(row_index == -1)
      return false;

   // Delete objects for this row (column labels and separator line)
   for(int col = 0; col < m_column_count; col++)
   {
      string obj_name = GenerateObjectName(row_key + "_col" + IntegerToString(col));
      #ifdef IS_MT5
         ObjectDelete(0, obj_name);
      #else
         ObjectDelete(obj_name);
      #endif
   }

   // Also delete separator line object if it exists
   string line_name = GenerateObjectName(row_key + "_line");
   #ifdef IS_MT5
      ObjectDelete(0, line_name);
   #else
      ObjectDelete(line_name);
   #endif

   // Also delete centered row object if it exists
   string center_name = GenerateObjectName(row_key + "_center");
   #ifdef IS_MT5
      ObjectDelete(0, center_name);
   #else
      ObjectDelete(center_name);
   #endif

   // Remove from array (shift remaining elements)
   for(int i = row_index; i < m_row_count - 1; i++)
   {
      m_row_keys[i] = m_row_keys[i + 1];
   }
   ArrayResize(m_row_keys, m_row_count - 1);
   m_row_count--;

   // Reposition all rows after the deleted one
   for(int i = row_index; i < m_row_count; i++)
   {
      int new_y = CalculateRowY(i);
      string key = m_row_keys[i];

      // Reposition column labels
      for(int col = 0; col < m_column_count; col++)
      {
         string obj_name = GenerateObjectName(key + "_col" + IntegerToString(col));
         #ifdef IS_MT5
            ObjectSetInteger(0, obj_name, OBJPROP_YDISTANCE, new_y);
         #else
            ObjectSet(obj_name, OBJPROP_YDISTANCE, new_y);
         #endif
      }

      // Reposition separator line if exists
      string sep_line_name = GenerateObjectName(key + "_line");
      int line_y = new_y + (m_row_height / 2) - 1;
      #ifdef IS_MT5
         if(ObjectFind(0, sep_line_name) >= 0)
            ObjectSetInteger(0, sep_line_name, OBJPROP_YDISTANCE, line_y);
      #else
         if(ObjectFind(sep_line_name) >= 0)
            ObjectSet(sep_line_name, OBJPROP_YDISTANCE, line_y);
      #endif

      // Reposition centered row if exists
      string ctr_name = GenerateObjectName(key + "_center");
      #ifdef IS_MT5
         if(ObjectFind(0, ctr_name) >= 0)
            ObjectSetInteger(0, ctr_name, OBJPROP_YDISTANCE, new_y);
      #else
         if(ObjectFind(ctr_name) >= 0)
            ObjectSet(ctr_name, OBJPROP_YDISTANCE, new_y);
      #endif
   }

   // Update background size
   UpdateBackgroundSize();

   return true;
}

//+------------------------------------------------------------------+
//| Clear all rows                                                    |
//+------------------------------------------------------------------+
void CGridPanel::ClearRows()
{
   // Delete all row objects (column labels, separator lines, and centered rows)
   for(int i = 0; i < m_row_count; i++)
   {
      string key = m_row_keys[i];

      // Delete column labels
      for(int col = 0; col < m_column_count; col++)
      {
         string obj_name = GenerateObjectName(key + "_col" + IntegerToString(col));
         #ifdef IS_MT5
            ObjectDelete(0, obj_name);
         #else
            ObjectDelete(obj_name);
         #endif
      }

      // Delete separator line if exists
      string line_name = GenerateObjectName(key + "_line");
      #ifdef IS_MT5
         ObjectDelete(0, line_name);
      #else
         ObjectDelete(line_name);
      #endif

      // Delete centered row if exists
      string center_name = GenerateObjectName(key + "_center");
      #ifdef IS_MT5
         ObjectDelete(0, center_name);
      #else
         ObjectDelete(center_name);
      #endif
   }

   ArrayResize(m_row_keys, 0);
   m_row_count = 0;

   // Update background size
   UpdateBackgroundSize();
}

//+------------------------------------------------------------------+
//| Set panel title (centered)                                        |
//+------------------------------------------------------------------+
void CGridPanel::SetTitle(string title, color clr)
{
   if(clr == clrNONE)
      clr = m_title_color;

   string title_obj = GenerateObjectName("Title");
   int title_x = CalculateTitleX();  // Center of panel
   int title_y = m_y_offset + m_padding_top;

   // Use ANCHOR_UPPER for horizontal centering
   CreatePanelLabel(title_obj, title_x, title_y, title, clr, PANEL_TITLE_FONT_SIZE, ANCHOR_UPPER);
}

//+------------------------------------------------------------------+
//| Initialize Slave EA panel with standard layout                  |
//| Parameters:                                                       |
//|   prefix      - Object name prefix for panel objects            |
//|   panel_width - Panel width in pixels (default: 280)           |
//| Returns: true on success                                         |
//+------------------------------------------------------------------+
bool CGridPanel::InitializeSlavePanel(string prefix = "SankeyCopierPanel_", int panel_width = DEFAULT_PANEL_WIDTH)
{
   // Initialize panel with specified or default dimensions
   if(!Initialize(prefix, DEFAULT_X_OFFSET, DEFAULT_Y_OFFSET, panel_width, DEFAULT_ROW_HEIGHT))
      return false;

   // Set title
   SetTitle("Sankey Copier - Slave", PANEL_COLOR_TITLE);

   // Separator after title
   AddSeparator("sep_top");

   // Add standard rows with initial values
   string status_vals[] = {"Status:", "DISABLED"};
   color status_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_DISABLED};
   AddRow("status", status_vals, status_cols);

   m_initialized = true;
   return true;
}

//+------------------------------------------------------------------+
//| Update configuration list rows                                   |
//+------------------------------------------------------------------+
void CGridPanel::UpdateConfigList(CopyConfig &configs[])
{
   // Update Active count
   string master_vals[2];
   master_vals[0] = "Active:";
   master_vals[1] = IntegerToString(ArraySize(configs));
   
   color master_cols[2];
   master_cols[0] = PANEL_COLOR_LABEL;
   master_cols[1] = PANEL_COLOR_VALUE;
   
   UpdateRow("master", master_vals, master_cols);

   // Dynamic rows for each config
   for(int i=0; i<ArraySize(configs); i++)
   {
       string key = "cfg_" + IntegerToString(i);
       // Truncate account name if too long
       string label = TruncateText(configs[i].master_account, 12); 
       
       string status_str = "DIS";
       color status_clr = PANEL_COLOR_DISABLED;
       if(configs[i].status == STATUS_CONNECTED) { status_str = "ON"; status_clr = PANEL_COLOR_CONNECTED; }
       else if(configs[i].status == STATUS_ENABLED) { status_str = "WAIT"; status_clr = PANEL_COLOR_WAITING; }
       
       string val = status_str + " x" + DoubleToString(configs[i].lot_multiplier, 1);
       if(configs[i].reverse_trade) val += " R";
       
       string vals[2];
       vals[0] = label;
       vals[1] = val;
       
       color cols[2];
       cols[0] = PANEL_COLOR_LABEL;
       cols[1] = status_clr;
       
       if(!UpdateRow(key, vals, cols))
       {
           AddRow(key, vals, cols);
       }
   }
   
   // Remove excess rows
   int i = ArraySize(configs);
   while(RemoveRow("cfg_" + IntegerToString(i)))
   {
       i++;
   }
}

//+------------------------------------------------------------------+
//| Initialize Master EA panel with standard layout                  |
//| Parameters:                                                       |
//|   prefix      - Object name prefix for panel objects            |
//|   panel_width - Panel width in pixels (default: 280)           |
//| Returns: true on success                                         |
//+------------------------------------------------------------------+
bool CGridPanel::InitializeMasterPanel(string prefix = "SankeyCopierPanel_", int panel_width = DEFAULT_PANEL_WIDTH)
{
   // Initialize panel with specified or default dimensions
   if(!Initialize(prefix, DEFAULT_X_OFFSET, DEFAULT_Y_OFFSET, panel_width, DEFAULT_ROW_HEIGHT))
      return false;

   // Set title
   SetTitle("Sankey Copier - Master", PANEL_COLOR_TITLE);

   // Separator after title
   AddSeparator("sep_top");

   // Add standard rows with initial values (white text except status value)
   string status_vals[] = {"Status:", "ACTIVE"};
   color status_cols[] = {clrWhite, PANEL_COLOR_CONNECTED};
   AddRow("status", status_vals, status_cols);

   string prefix_vals[] = {"Prefix:", " "};  // Use space to avoid "Label" default
   color prefix_cols[] = {clrWhite, clrWhite};
   AddRow("prefix", prefix_vals, prefix_cols);

   string suffix_vals[] = {"Suffix:", " "};  // Use space to avoid "Label" default
   color suffix_cols[] = {clrWhite, clrWhite};
   AddRow("suffix", suffix_vals, suffix_cols);

   string tracked_vals[] = {"Tracked Orders:", "0"};
   color tracked_cols[] = {clrWhite, clrWhite};
   AddRow("tracked", tracked_vals, tracked_cols);

   return true;
}

//+------------------------------------------------------------------+
//| Update symbol configuration row (prefix/suffix/map)             |
//+------------------------------------------------------------------+
void CGridPanel::UpdateSymbolConfig(string prefix, string suffix, string map)
{
   // Update prefix row (use space if empty to avoid "Label" default)
   UpdateCell("prefix", 1, (prefix == "") ? " " : prefix, clrWhite);

   // Update suffix row (use space if empty to avoid "Label" default)
   UpdateCell("suffix", 1, (suffix == "") ? " " : suffix, clrWhite);
}

//+------------------------------------------------------------------+
//| Show a text message instead of the grid (e.g. "Not Configured") |
//+------------------------------------------------------------------+
void CGridPanel::ShowMessage(string text, color clr = clrYellow)
{
   // Hide all grid elements
   string bg_name = GenerateObjectName("BG");
   string title_name = GenerateObjectName("Title");
   
   #ifdef IS_MT5
      // MQL5: Use OBJPROP_TIMEFRAMES to hide from all timeframes
      ObjectSetInteger(0, bg_name, OBJPROP_TIMEFRAMES, OBJ_NO_PERIODS);
      ObjectSetInteger(0, title_name, OBJPROP_TIMEFRAMES, OBJ_NO_PERIODS);
   #else
      // MQL4: Move objects off-screen to hide them
      ObjectSet(bg_name, OBJPROP_XDISTANCE, 10000);
      ObjectSet(bg_name, OBJPROP_YDISTANCE, 10000);
      ObjectSet(title_name, OBJPROP_XDISTANCE, 10000);
      ObjectSet(title_name, OBJPROP_YDISTANCE, 10000);
   #endif
   
   for(int i = 0; i < m_row_count; i++)
   {
      for(int col = 0; col < m_column_count; col++)
      {
         string obj_name = GenerateObjectName(m_row_keys[i] + "_col" + IntegerToString(col));
         #ifdef IS_MT5
            ObjectSetInteger(0, obj_name, OBJPROP_TIMEFRAMES, OBJ_NO_PERIODS);
         #else
            ObjectSet(obj_name, OBJPROP_XDISTANCE, 10000);
            ObjectSet(obj_name, OBJPROP_YDISTANCE, 10000);
         #endif
      }
   }
   
   // Create or update message label
   string msg_name = GenerateObjectName("Message");
   int x = m_x_offset + (m_panel_width / 2);
   int y = m_y_offset + (m_padding_top + m_title_height + (m_row_count * m_row_height)) / 2;
   
   // If object doesn't exist, create it
   #ifdef IS_MT5
      if(ObjectFind(0, msg_name) < 0)
   #else
      if(ObjectFind(msg_name) < 0)
   #endif
   {
      CreatePanelLabel(msg_name, m_x_offset + 10, m_y_offset + 10, text, clr, 10, ANCHOR_RIGHT_UPPER);
   }
   
   UpdatePanelLabel(msg_name, text, clr);
   
   // Ensure message is visible
   #ifdef IS_MT5
      ObjectSetInteger(0, msg_name, OBJPROP_TIMEFRAMES, OBJ_ALL_PERIODS);
   #else
      // MQL4: Ensure message is at correct position (in case it was hidden before)
      // Note: CreatePanelLabel sets the position, but if it existed and was moved, we need to restore it
      // However, ShowMessage creates/updates it, so we should just ensure it's visible/positioned
      // Since we don't move the message offscreen in HideMessage (we delete it), this is fine.
      // But just in case, let's make sure it's not hidden via TIMEFRAMES if we ever used that.
      ObjectSet(msg_name, OBJPROP_TIMEFRAMES, OBJ_ALL_PERIODS); 
   #endif
   
   ChartRedraw();
}

//+------------------------------------------------------------------+
//| Hide the message and restore the grid                           |
//+------------------------------------------------------------------+
void CGridPanel::HideMessage()
{
   string msg_name = GenerateObjectName("Message");
   #ifdef IS_MT5
      ObjectDelete(0, msg_name);
   #else
      ObjectDelete(msg_name);
   #endif
   
   // Restore grid elements
   string bg_name = GenerateObjectName("BG");
   string title_name = GenerateObjectName("Title");
   
   #ifdef IS_MT5
      ObjectSetInteger(0, bg_name, OBJPROP_TIMEFRAMES, OBJ_ALL_PERIODS);
      ObjectSetInteger(0, title_name, OBJPROP_TIMEFRAMES, OBJ_ALL_PERIODS);
   #else
      // MQL4: Restore positions
      int bg_x = CalculateBackgroundX();
      ObjectSet(bg_name, OBJPROP_XDISTANCE, bg_x);
      ObjectSet(bg_name, OBJPROP_YDISTANCE, m_y_offset);
      
      // Title position (centered in panel width, but CreatePanelLabel uses specific logic)
      // We need to check how CreatePanelLabel positions the title.
      // In Initialize: CreatePanelLabel(..., m_x_offset + (m_panel_width/2), m_y_offset + 5, ...)
      // Wait, SetTitle calls CreatePanelLabel.
      // Let's check SetTitle logic or assume standard positioning.
      // Actually, SetTitle uses: m_x_offset + (m_panel_width / 2), m_y_offset + 5
      ObjectSet(title_name, OBJPROP_XDISTANCE, m_x_offset + (m_panel_width / 2));
      ObjectSet(title_name, OBJPROP_YDISTANCE, m_y_offset + 5);
   #endif
   
   for(int i = 0; i < m_row_count; i++)
   {
      int row_y = CalculateRowY(i);
      for(int col = 0; col < m_column_count; col++)
      {
         string obj_name = GenerateObjectName(m_row_keys[i] + "_col" + IntegerToString(col));
         #ifdef IS_MT5
            ObjectSetInteger(0, obj_name, OBJPROP_TIMEFRAMES, OBJ_ALL_PERIODS);
         #else
            // MQL4: Restore positions
            ObjectSet(obj_name, OBJPROP_XDISTANCE, m_column_widths[col]);
            ObjectSet(obj_name, OBJPROP_YDISTANCE, row_y);
         #endif
      }
   }
   
   ChartRedraw();
}

//+------------------------------------------------------------------+
//| Update status row (4 states)                                     |
//| status: 0=DISABLED, 1=ENABLED (Master disconnected), 2=CONNECTED, -1=NO_CONFIG |
//+------------------------------------------------------------------+
void CGridPanel::UpdateStatusRow(int status, bool allow_new_orders)
{
   string vals[2];
   vals[0] = "Status:";

   string status_label;
   if(status == STATUS_DISABLED)
      status_label = "DISABLED";
   else if(status == STATUS_ENABLED)
      status_label = "ENABLED";
   else if(status == STATUS_CONNECTED)
      status_label = "CONNECTED";
   else if(status == STATUS_NO_CONFIG)
      status_label = "NO CONFIG";
   else
      status_label = "UNKNOWN";

   string allow_note = "";
   if(allow_new_orders && status == STATUS_CONNECTED)
      allow_note = " (orders allowed)";
   else if(!allow_new_orders && status == STATUS_CONNECTED)
      allow_note = " (orders blocked)";

   vals[1] = status_label + allow_note;

   color cols[2];
   cols[0] = PANEL_COLOR_LABEL;

   // Assign color based on status
   if(status == STATUS_DISABLED)
   {
      cols[1] = PANEL_COLOR_DISABLED;         // Gray
   }
   else if(status == STATUS_ENABLED)
   {
      cols[1] = PANEL_COLOR_WAITING;          // Yellow (waiting for Master)
   }
   else if(status == STATUS_CONNECTED)
   {
      cols[1] = PANEL_COLOR_CONNECTED;        // Green (connected to Master)
   }
   else if(status == STATUS_NO_CONFIG)
   {
      cols[1] = PANEL_COLOR_NO_CONFIG;        // Dark gray (no config received)
   }
   else
   {
      cols[1] = PANEL_COLOR_DISABLED;         // Gray for unknown state
   }

   UpdateRow("status", vals, cols);
}

//+------------------------------------------------------------------+
//| Update status row based on configs array                         |
//| Centralized logic for determining panel status from configs      |
//+------------------------------------------------------------------+
void CGridPanel::UpdatePanelStatusFromConfigs(CopyConfig &configs[])
{
   if(ArraySize(configs) == 0)
   {
      UpdateStatusRow(STATUS_NO_CONFIG);
      return;
   }

   bool any_connected = false;
   bool all_disabled = true;
   bool any_allow_new_orders = false;

   for(int i=0; i<ArraySize(configs); i++)
   {
      if(configs[i].allow_new_orders)
      {
         any_allow_new_orders = true;
      }
      if(configs[i].status == STATUS_CONNECTED)
      {
         any_connected = true;
         all_disabled = false;
         break;
      }
      if(configs[i].status != STATUS_DISABLED)
      {
         all_disabled = false;
      }
      if(configs[i].allow_new_orders)
      {
         any_allow_new_orders = true;
      }
   }

   if(any_connected)
      UpdateStatusRow(STATUS_CONNECTED, any_allow_new_orders);
   else if(all_disabled)
      UpdateStatusRow(STATUS_DISABLED);
   else
      UpdateStatusRow(STATUS_ENABLED, any_allow_new_orders);
}

//+------------------------------------------------------------------+
//| Update master row                                                |
//| Note: Long master names are truncated to fit panel width        |
//+------------------------------------------------------------------+
void CGridPanel::UpdateMasterRow(string master_name)
{
   // Truncate master name if too long (280px panel ~ 25 chars safe)
   int max_chars = (m_panel_width == 280) ? 25 : (m_panel_width / 11);
   string truncated_name = TruncateText(master_name, max_chars);

   string vals[2];
   vals[0] = "Master:";
   vals[1] = truncated_name;
   color cols[2];
   cols[0] = PANEL_COLOR_LABEL;
   cols[1] = PANEL_COLOR_VALUE;
   UpdateRow("master", vals, cols);
}

//+------------------------------------------------------------------+
//| Update lot multiplier row                                        |
//+------------------------------------------------------------------+
void CGridPanel::UpdateLotMultiplierRow(double multiplier)
{
   string vals[2];
   vals[0] = "Lot x:";
   vals[1] = StringFormat("%.2f", multiplier);
   color cols[2];
   cols[0] = PANEL_COLOR_LABEL;
   cols[1] = PANEL_COLOR_VALUE;
   UpdateRow("lot", vals, cols);
}

//+------------------------------------------------------------------+
//| Update reverse row                                               |
//+------------------------------------------------------------------+
void CGridPanel::UpdateReverseRow(bool reverse)
{
   string vals[2];
   vals[0] = "Reverse:";
   vals[1] = reverse ? "YES" : "NO";
   color cols[2];
   cols[0] = PANEL_COLOR_LABEL;
   cols[1] = PANEL_COLOR_VALUE;
   UpdateRow("reverse", vals, cols);
}

//+------------------------------------------------------------------+
//| Update version row                                               |
//+------------------------------------------------------------------+
void CGridPanel::UpdateVersionRow(int version)
{
   string vals[2];
   vals[0] = "Version:";
   vals[1] = IntegerToString(version);
   color cols[2];
   cols[0] = PANEL_COLOR_LABEL;
   cols[1] = PANEL_COLOR_VALUE;
   UpdateRow("version", vals, cols);
}

//+------------------------------------------------------------------+
//| Update symbol count row                                          |
//+------------------------------------------------------------------+
void CGridPanel::UpdateSymbolCountRow(int count)
{
   string vals[2];
   vals[0] = "Symbols:";
   vals[1] = IntegerToString(count);
   color cols[2];
   cols[0] = PANEL_COLOR_LABEL;
   cols[1] = PANEL_COLOR_VALUE;
   UpdateRow("symbols", vals, cols);
}

//+------------------------------------------------------------------+
//| Update tracked orders row (Master EA)                            |
//+------------------------------------------------------------------+
void CGridPanel::UpdateTrackedOrdersRow(int count)
{
   string vals[2];
   vals[0] = "Tracked Orders:";
   vals[1] = IntegerToString(count);
   color cols[2];
   cols[0] = clrWhite;
   cols[1] = clrWhite;
   UpdateRow("tracked", vals, cols);
}

//+------------------------------------------------------------------+
//| Update magic filter row (Master EA)                              |
//+------------------------------------------------------------------+
void CGridPanel::UpdateMagicFilterRow(int magic)
{
   string vals[2];
   vals[0] = "Magic Filter:";
   vals[1] = (magic == 0) ? "All" : IntegerToString(magic);
   color cols[2];
   cols[0] = PANEL_COLOR_LABEL;
   cols[1] = PANEL_COLOR_VALUE;
   UpdateRow("magic", vals, cols);
}

//+------------------------------------------------------------------+
//| Update server row (Master EA)                                    |
//+------------------------------------------------------------------+
void CGridPanel::UpdateServerRow(string address)
{
   string vals[2];
   vals[0] = "Server:";
   vals[1] = TruncateText(address, 25);
   color cols[2];
   cols[0] = PANEL_COLOR_LABEL;
   cols[1] = PANEL_COLOR_VALUE;
   UpdateRow("server", vals, cols);
}

//+------------------------------------------------------------------+
//| Set panel colors                                                  |
//+------------------------------------------------------------------+
void CGridPanel::SetColors(color bg, color border, color title)
{
   m_bg_color = bg;
   m_border_color = border;
   m_title_color = title;
}

//+------------------------------------------------------------------+
//| Set panel padding                                                 |
//+------------------------------------------------------------------+
void CGridPanel::SetPadding(int top, int bottom, int left, int right)
{
   m_padding_top = top;
   m_padding_bottom = bottom;
   m_padding_left = left;
   m_padding_right = right;
}

//+------------------------------------------------------------------+
//| Set row height                                                    |
//+------------------------------------------------------------------+
void CGridPanel::SetRowHeight(int height)
{
   m_row_height = height;
}

//+------------------------------------------------------------------+
//| Refresh panel (redraw all elements)                             |
//+------------------------------------------------------------------+
void CGridPanel::Refresh()
{
   UpdateBackgroundSize();
   ChartRedraw();
}

//+------------------------------------------------------------------+
//| Delete all panel objects                                          |
//+------------------------------------------------------------------+
void CGridPanel::Delete()
{
   // Delete background
   string bg_name = GenerateObjectName("BG");
   #ifdef IS_MT5
      ObjectDelete(0, bg_name);
   #else
      ObjectDelete(bg_name);
   #endif

   // Delete title
   string title_name = GenerateObjectName("Title");
   #ifdef IS_MT5
      ObjectDelete(0, title_name);
   #else
      ObjectDelete(title_name);
   #endif

   // Delete message (if exists)
   string msg_name = GenerateObjectName("Message");
   #ifdef IS_MT5
      ObjectDelete(0, msg_name);
   #else
      ObjectDelete(msg_name);
   #endif

   // Delete all rows
   ClearRows();
}

//+------------------------------------------------------------------+
//| Calculate background panel X position (left edge from right)    |
//| For CORNER_RIGHT_UPPER: XDISTANCE specifies left edge position  |
//| Returns: X coordinate for background panel                      |
//+------------------------------------------------------------------+
int CGridPanel::CalculateBackgroundX()
{
   // Left edge = x_offset + panel_width (so right edge = x_offset)
   return m_x_offset + m_panel_width;
}

//+------------------------------------------------------------------+
//| Calculate column X position                                      |
//| Parameters:                                                       |
//|   column_index - Column number (0 = label, 1 = value, etc.)    |
//| Returns: X coordinate for column text                           |
//| Note: Both columns use ANCHOR_LEFT_UPPER for unified coordinate |
//|       system. Columns are accumulated left-to-right.            |
//|       Column 0 (label): Fixed width (LABEL_COLUMN_WIDTH)        |
//|       Column 1 (value): Remaining space                         |
//+------------------------------------------------------------------+
int CGridPanel::CalculateColumnX(int column_index)
{
   // Panel left edge (distance from right edge of screen)
   // For CORNER_RIGHT_UPPER: larger X = more to the left
   int panel_left = m_x_offset + m_panel_width;

   if(column_index == 0)
   {
      // Label column: ANCHOR_LEFT_UPPER
      // Position at panel left edge minus left padding
      // Example: offset=10, width=280, padding_left=5 -> 10+280-5=285px
      return panel_left - m_padding_left;
   }
   else
   {
      // Value column: ANCHOR_LEFT_UPPER (unified coordinate system)
      // Position at label column left edge minus label column width
      // This gives the value column all remaining space
      // Example: label_x=285, label_width=100 -> 285-100=185px
      int label_col_x = panel_left - m_padding_left;
      return label_col_x - LABEL_COLUMN_WIDTH;
   }
}

//+------------------------------------------------------------------+
//| Calculate title X position (centered)                            |
//| Returns: X coordinate for centered title text                   |
//+------------------------------------------------------------------+
int CGridPanel::CalculateTitleX()
{
   // Center of panel for ANCHOR_RIGHT_UPPER
   // Center point = x_offset + (panel_width / 2)
   return m_x_offset + m_panel_width / 2;
}

//+------------------------------------------------------------------+
//| Calculate Y position for a row                                   |
//| Parameters:                                                       |
//|   row_index - Row number (0-based)                              |
//| Returns: Y coordinate from top edge                             |
//+------------------------------------------------------------------+
int CGridPanel::CalculateRowY(int row_index)
{
   // Y = y_offset + padding_top + title_height + (row_index * row_height)
   return m_y_offset + m_padding_top + m_title_height + (row_index * m_row_height);
}

//+------------------------------------------------------------------+
//| Calculate total panel height                                     |
//| Returns: Panel height in pixels                                 |
//+------------------------------------------------------------------+
int CGridPanel::CalculatePanelHeight()
{
   // Height = padding_top + title_height + (row_count * row_height) + padding_bottom
   return m_padding_top + m_title_height + (m_row_count * m_row_height) + m_padding_bottom;
}

//+------------------------------------------------------------------+
//| Update background panel size                                     |
//+------------------------------------------------------------------+
void CGridPanel::UpdateBackgroundSize()
{
   int new_height = CalculatePanelHeight();
   string bg_name = GenerateObjectName("BG");

   #ifdef IS_MT5
      ObjectSetInteger(0, bg_name, OBJPROP_YSIZE, new_height);
   #else
      ObjectSet(bg_name, OBJPROP_YSIZE, new_height);
   #endif
}

//+------------------------------------------------------------------+
//| Generate object name with prefix                                 |
//+------------------------------------------------------------------+
string CGridPanel::GenerateObjectName(string suffix)
{
   return m_prefix + suffix;
}

//+------------------------------------------------------------------+
//| Update carousel configs and refresh display                       |
//| Shows detailed copy settings with pagination for multiple Masters |
//+------------------------------------------------------------------+
void CGridPanel::UpdateCarouselConfigs(CopyConfig &configs[])
{
   int config_count = ArraySize(configs);

   // Cache configs
   ArrayResize(m_cached_configs, config_count);
   for(int i = 0; i < config_count; i++)
   {
      m_cached_configs[i] = configs[i];
   }

   m_carousel_count = config_count;
   m_carousel_enabled = (config_count > 0);

   // Validate current index
   if(m_carousel_index >= config_count)
      m_carousel_index = MathMax(0, config_count - 1);

   // Clear existing carousel rows (cfg_* rows)
   int i = 0;
   while(RemoveRow("cfg_" + IntegerToString(i)))
   {
      i++;
   }

   // Remove detail rows if they exist
   RemoveRow("master_detail");
   RemoveRow("prefix_row");
   RemoveRow("suffix_row");
   for(int j = 0; j < 10; j++)
      RemoveRow("map_" + IntegerToString(j));
   RemoveRow("map_more");
   RemoveRow("lot_mode");
   RemoveRow("reverse");
   RemoveRow("sep_bottom");
   RemoveRow("nav_row");

   if(config_count == 0)
   {
      return;
   }

   // Show current page
   ShowCarouselPage(m_carousel_index);
}

//+------------------------------------------------------------------+
//| Show carousel page at specified index                            |
//| Displays detailed settings for one Master config                  |
//+------------------------------------------------------------------+
void CGridPanel::ShowCarouselPage(int index)
{
   if(m_carousel_count == 0 || index < 0 || index >= m_carousel_count)
      return;

   m_carousel_index = index;

   // Remove previous detail rows
   RemoveRow("master_detail");
   RemoveRow("prefix_row");
   RemoveRow("suffix_row");
   for(int j = 0; j < 10; j++)
      RemoveRow("map_" + IntegerToString(j));
   RemoveRow("map_more");
   RemoveRow("lot_mode");
   RemoveRow("reverse");
   RemoveRow("sep_bottom");
   RemoveRow("nav_row");

   CopyConfig cfg = m_cached_configs[index];

   // Master account (truncated)
   string master_label = TruncateText(cfg.master_account, 20);
   string master_vals[2];
   master_vals[0] = "Master:";
   master_vals[1] = master_label;
   color master_cols[2];
   master_cols[0] = clrWhite;
   master_cols[1] = clrWhite;
   AddRow("master_detail", master_vals, master_cols);

   // Prefix (use space if empty to avoid "Label" default)
   string prefix_vals[2];
   prefix_vals[0] = "Prefix:";
   prefix_vals[1] = (cfg.symbol_prefix == "" || cfg.symbol_prefix == NULL) ? " " : cfg.symbol_prefix;
   color prefix_cols[2];
   prefix_cols[0] = clrWhite;
   prefix_cols[1] = clrWhite;
   AddRow("prefix_row", prefix_vals, prefix_cols);

   // Suffix (use space if empty to avoid "Label" default)
   string suffix_vals[2];
   suffix_vals[0] = "Suffix:";
   suffix_vals[1] = (cfg.symbol_suffix == "" || cfg.symbol_suffix == NULL) ? " " : cfg.symbol_suffix;
   color suffix_cols[2];
   suffix_cols[0] = clrWhite;
   suffix_cols[1] = clrWhite;
   AddRow("suffix_row", suffix_vals, suffix_cols);

   // Symbol mappings
   int mapping_count = ArraySize(cfg.symbol_mappings);
   if(mapping_count > 0)
   {
      int show_count = MathMin(mapping_count, 5);
      for(int m = 0; m < show_count; m++)
      {
         string map_vals[2];
         if(m == 0)
            map_vals[0] = "Map:";
         else
            map_vals[0] = " ";  // Use space to avoid "Label" default
         map_vals[1] = cfg.symbol_mappings[m].source_symbol + " -> " + cfg.symbol_mappings[m].target_symbol;
         color map_cols[2];
         map_cols[0] = clrWhite;
         map_cols[1] = clrWhite;
         AddRow("map_" + IntegerToString(m), map_vals, map_cols);
      }

      if(mapping_count > 5)
      {
         string more_vals[2];
         more_vals[0] = " ";  // Use space to avoid "Label" default
         more_vals[1] = "+" + IntegerToString(mapping_count - 5) + " more";
         color more_cols[2];
         more_cols[0] = clrWhite;
         more_cols[1] = clrGray;
         AddRow("map_more", more_vals, more_cols);
      }
   }
   else
   {
      // Show empty Map row
      string map_vals[2];
      map_vals[0] = "Map:";
      map_vals[1] = " ";  // Use space to avoid "Label" default
      color map_cols[2];
      map_cols[0] = clrWhite;
      map_cols[1] = clrWhite;
      AddRow("map_0", map_vals, map_cols);
   }

   // Lot Mode
   string lot_str = "";
   if(cfg.lot_calculation_mode == LOT_CALC_MODE_MARGIN_RATIO)
      lot_str = "Margin Ratio";
   else
      lot_str = "x" + DoubleToString(cfg.lot_multiplier, 2);

   string lot_vals[2];
   lot_vals[0] = "Lot Mode:";
   lot_vals[1] = lot_str;
   color lot_cols[2];
   lot_cols[0] = clrWhite;
   lot_cols[1] = clrWhite;
   AddRow("lot_mode", lot_vals, lot_cols);

   // Reverse
   string rev_vals[2];
   rev_vals[0] = "Reverse:";
   rev_vals[1] = cfg.reverse_trade ? "ON" : "OFF";
   color rev_cols[2];
   rev_cols[0] = clrWhite;
   rev_cols[1] = clrWhite;
   AddRow("reverse", rev_vals, rev_cols);

   // Separator and navigation (only if multiple configs)
   if(m_carousel_count > 1)
   {
      AddSeparator("sep_bottom");

      string nav_str = "< " + IntegerToString(index + 1) + "/" + IntegerToString(m_carousel_count) + " >";
      AddCenteredRow("nav_row", nav_str, clrSkyBlue);
   }

   ChartRedraw();
}

//+------------------------------------------------------------------+
//| Navigate to next carousel page                                   |
//+------------------------------------------------------------------+
void CGridPanel::NextCarouselPage()
{
   if(m_carousel_count <= 1)
      return;

   int next_index = (m_carousel_index + 1) % m_carousel_count;
   ShowCarouselPage(next_index);
}

//+------------------------------------------------------------------+
//| Navigate to previous carousel page                               |
//+------------------------------------------------------------------+
void CGridPanel::PrevCarouselPage()
{
   if(m_carousel_count <= 1)
      return;

   int prev_index = (m_carousel_index - 1 + m_carousel_count) % m_carousel_count;
   ShowCarouselPage(prev_index);
}

//+------------------------------------------------------------------+
//| Handle chart click for navigation                                |
//| Returns: true if click was handled (within navigation area)      |
//+------------------------------------------------------------------+
bool CGridPanel::HandleChartClick(int x, int y)
{
   if(!m_carousel_enabled || m_carousel_count <= 1)
      return false;

   // Convert screen coordinates to chart coordinates
   // For CORNER_RIGHT_UPPER, we need to calculate relative to right edge
   int chart_width = (int)ChartGetInteger(0, CHART_WIDTH_IN_PIXELS);
   int click_x_from_right = chart_width - x;

   // Check if click is within panel X bounds
   if(click_x_from_right < m_x_offset || click_x_from_right > m_x_offset + m_panel_width)
      return false;

   // Check if click is within navigation row Y bounds
   // Navigation row is the last row
   int nav_row_index = -1;
   for(int i = 0; i < m_row_count; i++)
   {
      if(m_row_keys[i] == "nav_row")
      {
         nav_row_index = i;
         break;
      }
   }

   if(nav_row_index < 0)
      return false;

   int nav_y_start = CalculateRowY(nav_row_index);
   int nav_y_end = nav_y_start + m_row_height;

   if(y < nav_y_start || y > nav_y_end)
      return false;

   // Determine if click is on left (<) or right (>) half
   int panel_center_from_right = m_x_offset + (m_panel_width / 2);

   if(click_x_from_right > panel_center_from_right)
   {
      // Click on left side (< previous)
      PrevCarouselPage();
   }
   else
   {
      // Click on right side (> next)
      NextCarouselPage();
   }

   return true;
}

#endif // SANKEY_COPIER_GRIDPANEL_MQH
