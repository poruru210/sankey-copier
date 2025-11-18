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
#define PANEL_COLOR_BG C'40,40,50'        // Dark background
#define PANEL_COLOR_BORDER clrDimGray      // Border color
#define PANEL_COLOR_TITLE clrWhite
#define PANEL_COLOR_LABEL clrLightGray
#define PANEL_COLOR_VALUE clrWhite
#define PANEL_COLOR_ENABLED clrLime
#define PANEL_COLOR_DISABLED clrRed

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
   m_column_count = 2;  // Default: 2 columns (label, value)
   m_prefix = "GridPanel_";
   m_x_offset = 10;
   m_y_offset = 20;
   m_panel_width = 200;
   m_row_height = 15;
   m_title_height = 20;
   m_padding_top = 3;
   m_padding_bottom = 5;
   m_padding_left = 5;
   m_padding_right = 10;
   m_bg_color = C'40,40,50';
   m_border_color = clrDimGray;
   m_title_color = clrWhite;
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
   m_prefix = prefix;
   m_x_offset = x_offset;
   m_y_offset = y_offset;
   m_panel_width = panel_width;
   m_row_height = row_height;
   m_title_height = row_height + 5;

   // Initialize row management arrays
   ArrayResize(m_row_keys, 0);
   m_row_count = 0;

   // Default 2-column layout
   ArrayResize(m_column_widths, 2);
   m_column_widths[0] = m_x_offset + m_panel_width - m_padding_left;   // Left column (labels)
   m_column_widths[1] = m_x_offset + m_padding_right;                  // Right column (values)

   // Create background with initial size (title only)
   // For CORNER_RIGHT_UPPER: XDISTANCE is the left edge position from right
   // To fit panel in screen: left_edge = x_offset + panel_width
   int initial_height = m_padding_top + m_title_height + m_padding_bottom;
   CreatePanelBackground(GenerateObjectName("BG"),
                        m_x_offset + m_panel_width,  // Left edge at 210px from right
                        m_y_offset,
                        m_panel_width,               // Width 200px (right edge at 10px)
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
                      8);
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
   int title_x = m_x_offset + m_panel_width / 2 + 50;  // Approximate center
   int title_y = m_y_offset + m_padding_top;

   CreatePanelLabel(title_obj, title_x, title_y, title, clr, 9);
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
