//+------------------------------------------------------------------+
//|                                   SankeyCopierGridPanel.mqh      |
//|                          Dynamic Grid Layout Panel for MT4/MT5   |
//|                                                                  |
//| Purpose: Provides dynamic grid layout with automatic row        |
//|          management, coordinate calculation, and panel sizing.  |
//|          Supports runtime addition/removal of data rows.        |
//+------------------------------------------------------------------+
#property copyright "Sankey Copier"
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
#define PANEL_COLOR_ENABLED clrLime
#define PANEL_COLOR_DISABLED clrRed

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

//+------------------------------------------------------------------+
//| Helper Functions for Panel Creation                              |
//+------------------------------------------------------------------+

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
      ObjectSetInteger(0, name, OBJPROP_BACK, false);
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

   // Row management
   int      m_row_count;           // Current number of data rows (excluding title)
   string   m_row_keys[];          // Row identifiers for lookups

   // Colors
   color    m_bg_color;
   color    m_border_color;
   color    m_title_color;

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
   bool     UpdateRow(string row_key, string &values[], color &colors[]);
   bool     UpdateCell(string row_key, int column_index, string value, color clr = -1);
   bool     RemoveRow(string row_key);
   void     ClearRows();
   int      GetRowCount() const { return m_row_count; }

   // Title management
   void     SetTitle(string title, color clr = -1);

   // Slave EA panel helpers (high-level update methods)
   bool     InitializeSlavePanel(string prefix = "SankeyCopierPanel_", int panel_width = DEFAULT_PANEL_WIDTH);
   void     UpdateStatusRow(bool enabled);
   void     UpdateMasterRow(string master_name);
   void     UpdateLotMultiplierRow(double multiplier);
   void     UpdateReverseRow(bool reverse);
   void     UpdateVersionRow(int version);
   void     UpdateSymbolCountRow(int count);

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

   // Default 2-column layout
   ArrayResize(m_column_widths, 2);
   m_column_widths[0] = CalculateColumnX(0);   // Left column (labels)
   m_column_widths[1] = CalculateColumnX(1);   // Right column (values)

   // Create background with initial size (title only)
   // For CORNER_RIGHT_UPPER: XDISTANCE is the left edge position from right
   // To fit panel in screen: left_edge = x_offset + panel_width
   int initial_height = m_padding_top + m_title_height + m_padding_bottom;
   CreatePanelBackground(GenerateObjectName("BG"),
                        CalculateBackgroundX(),  // Calculate left edge position
                        m_y_offset,
                        m_panel_width,
                        initial_height,
                        m_bg_color);

   Print("Grid panel initialized: ", m_prefix);
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

   // Create label objects for each column
   for(int col = 0; col < m_column_count; col++)
   {
      string obj_name = GenerateObjectName(row_key + "_col" + IntegerToString(col));
      CreatePanelLabel(obj_name,
                      m_column_widths[col],
                      row_y,
                      values[col],
                      colors[col],
                      PANEL_DATA_FONT_SIZE);
   }

   m_row_count++;

   // Update background height
   UpdateBackgroundSize();

   return m_row_count - 1;
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
      Print("Row '", row_key, "' not found");
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

   // Delete objects for this row
   for(int col = 0; col < m_column_count; col++)
   {
      string obj_name = GenerateObjectName(row_key + "_col" + IntegerToString(col));
      #ifdef IS_MT5
         ObjectDelete(0, obj_name);
      #else
         ObjectDelete(obj_name);
      #endif
   }

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

      for(int col = 0; col < m_column_count; col++)
      {
         string obj_name = GenerateObjectName(key + "_col" + IntegerToString(col));
         #ifdef IS_MT5
            ObjectSetInteger(0, obj_name, OBJPROP_YDISTANCE, new_y);
         #else
            ObjectSet(obj_name, OBJPROP_YDISTANCE, new_y);
         #endif
      }
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
   // Delete all row objects
   for(int i = 0; i < m_row_count; i++)
   {
      string key = m_row_keys[i];
      for(int col = 0; col < m_column_count; col++)
      {
         string obj_name = GenerateObjectName(key + "_col" + IntegerToString(col));
         #ifdef IS_MT5
            ObjectDelete(0, obj_name);
         #else
            ObjectDelete(obj_name);
         #endif
      }
   }

   ArrayResize(m_row_keys, 0);
   m_row_count = 0;

   // Update background size
   UpdateBackgroundSize();
}

//+------------------------------------------------------------------+
//| Set panel title                                                   |
//+------------------------------------------------------------------+
void CGridPanel::SetTitle(string title, color clr = -1)
{
   if(clr == -1)
      clr = m_title_color;

   string title_obj = GenerateObjectName("Title");
   int title_x = CalculateTitleX();  // Use encapsulated calculation
   int title_y = m_y_offset + m_padding_top;

   CreatePanelLabel(title_obj, title_x, title_y, title, clr, PANEL_TITLE_FONT_SIZE);
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

   // Add standard rows with initial values
   string status_vals[] = {"Status:", "DISABLED"};
   color status_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_DISABLED};
   AddRow("status", status_vals, status_cols);

   string master_vals[] = {"Master:", "N/A"};
   color master_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
   AddRow("master", master_vals, master_cols);

   string lot_vals[] = {"Lot x:", "N/A"};
   color lot_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
   AddRow("lot", lot_vals, lot_cols);

   string reverse_vals[] = {"Reverse:", "N/A"};
   color reverse_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
   AddRow("reverse", reverse_vals, reverse_cols);

   string version_vals[] = {"Version:", "N/A"};
   color version_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
   AddRow("version", version_vals, version_cols);

   string symbols_vals[] = {"Symbols:", "N/A"};
   color symbols_cols[] = {PANEL_COLOR_LABEL, PANEL_COLOR_VALUE};
   AddRow("symbols", symbols_vals, symbols_cols);

   return true;
}

//+------------------------------------------------------------------+
//| Update status row                                                |
//+------------------------------------------------------------------+
void CGridPanel::UpdateStatusRow(bool enabled)
{
   string vals[2];
   vals[0] = "Status:";
   vals[1] = enabled ? "ENABLED" : "DISABLED";
   color cols[2];
   cols[0] = PANEL_COLOR_LABEL;
   cols[1] = enabled ? PANEL_COLOR_ENABLED : PANEL_COLOR_DISABLED;
   UpdateRow("status", vals, cols);
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

   // Delete all rows
   ClearRows();

   Print("Grid panel deleted: ", m_prefix);
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
//|   column_index - Column number (0 = left, 1 = right, etc.)     |
//| Returns: X coordinate for column text                           |
//+------------------------------------------------------------------+
int CGridPanel::CalculateColumnX(int column_index)
{
   if(column_index == 0)
   {
      // Left column: labels aligned to left side of panel
      return m_x_offset + m_panel_width - m_padding_left;
   }
   else
   {
      // Right column(s): values aligned to right side of panel
      return m_x_offset + m_padding_right;
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
