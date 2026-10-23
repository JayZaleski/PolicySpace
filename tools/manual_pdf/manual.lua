-- Pandoc filter for the manual's PDF (see tools/manual_pdf/index.tex.j2).
--
-- Headings: the notebook's "#" title is typeset by the template, so it is
-- dropped here; "##" becomes \section, "###" \subsection, and so on. Each
-- heading's label is its text with spaces as hyphens, which is the anchor the
-- notebook's links use ([Markets](#Markets)), so those links work in the PDF.
--
-- Tables: header cells are set bold, the table in the manual's table font
-- (smaller for wide tables), and code in cells may break after an underscore,
-- so names like unimproved_copper fit narrow columns.
--
-- Indented code blocks (the ones without a language) get the same washed box
-- as fenced ones, with lines longer than the box wrapped the way the manual
-- wraps them by hand: the continuation aligned under the line's second column.

local WRAP = 74

function Header(el)
  if el.level == 1 then
    return {}
  end
  el.level = el.level - 1
  el.identifier = pandoc.utils.stringify(el):gsub(" ", "-")
  return el
end

local function breakable_code(code)
  local text = code.text:gsub("[\\{}]", function(c) return "\\" .. c end)
  text = text:gsub("_", "\\_\\allowbreak{}")
  return pandoc.RawInline("latex", "\\texttt{" .. text .. "}")
end

-- When pandoc gives a table relative column widths (it does when a row is
-- wider than its column layout), it shares the width equally, which squeezes
-- long columns and wastes short ones. Reshare it by the columns' content:
-- each column needs at least its longest word, and wants up to its longest
-- cell, capped so one wordy column cannot take everything.
local CELL_CAP = 28
local WORD_WEIGHT = 1.4 -- bold headers and code run wider than the character count

local function size_columns(tbl)
  local sized = false
  for _, spec in ipairs(tbl.colspecs) do
    if spec[2] ~= nil and spec[2] ~= "ColWidthDefault" then
      sized = true
    end
  end
  if not sized then
    return
  end
  local want = {}
  local function measure(rows)
    for _, row in ipairs(rows) do
      for i, cell in ipairs(row.cells) do
        local text = pandoc.utils.stringify(cell.contents)
        local longest_word = 0
        for word in text:gmatch("[^%s_]+") do
          longest_word = math.max(longest_word, #word)
        end
        local need = math.max(longest_word * WORD_WEIGHT, math.min(#text, CELL_CAP))
        want[i] = math.max(want[i] or 0, need)
      end
    end
  end
  measure(tbl.head.rows)
  for _, body in ipairs(tbl.bodies) do
    measure(body.body)
  end
  local total = 0
  for i = 1, #tbl.colspecs do
    want[i] = math.max(want[i] or 1, 3)
    total = total + want[i]
  end
  for i, spec in ipairs(tbl.colspecs) do
    tbl.colspecs[i] = { spec[1], want[i] / total }
  end
end

function Table(el)
  for _, row in ipairs(el.head.rows) do
    for _, cell in ipairs(row.cells) do
      for i, block in ipairs(cell.contents) do
        if block.t == "Plain" or block.t == "Para" then
          cell.contents[i] = pandoc.Plain({ pandoc.Strong(block.content) })
        end
      end
    end
  end
  size_columns(el) -- before the code spans become raw LaTeX, which measures as empty
  el = el:walk({ Code = breakable_code })
  local font = #el.colspecs >= 6 and "\\tablefontwide" or "\\tablefont"
  return {
    pandoc.RawBlock("latex", "\\begingroup" .. font),
    el,
    pandoc.RawBlock("latex", "\\endgroup"),
  }
end

local function wrap_line(line)
  if #line <= WRAP then
    return { line }
  end
  local lead, first, gap = line:match("^(%s*)(%S+)(%s+)")
  local indent = #lead + 4
  if gap and #gap >= 2 then
    indent = #lead + #first + #gap
  end
  local out = {}
  local rest = line
  while #rest > WRAP do
    local cut = rest:sub(1, WRAP):match("^.*()%s") -- the last space within WRAP
    if not cut or cut <= indent then
      break
    end
    table.insert(out, rest:sub(1, cut - 1))
    rest = string.rep(" ", indent) .. rest:sub(cut + 1):gsub("^%s+", "")
  end
  table.insert(out, rest)
  return out
end

function CodeBlock(el)
  if #el.classes > 0 then
    return nil
  end
  local lines = {}
  for line in (el.text .. "\n"):gmatch("(.-)\n") do
    for _, piece in ipairs(wrap_line(line)) do
      table.insert(lines, piece)
    end
  end
  local text = table.concat(lines, "\n"):gsub("[\\{}]", function(c) return "\\" .. c end)
  return pandoc.RawBlock("latex",
    "\\begin{Shaded}\n\\begin{Highlighting}[]\n" .. text .. "\n\\end{Highlighting}\n\\end{Shaded}")
end
