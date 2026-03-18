-- Pandoc Lua filter for inline Gleam syntax highlighting.
-- Usage in markdown: `code here`{.gleam}

local keywords = {
  ["as"]=true, ["case"]=true, ["const"]=true, ["else"]=true,
  ["fn"]=true, ["if"]=true, ["import"]=true, ["let"]=true,
  ["opaque"]=true, ["pub"]=true, ["type"]=true, ["use"]=true,
  ["assert"]=true, ["panic"]=true, ["todo"]=true,
}

local types = {
  ["Int"]=true, ["Float"]=true, ["Bool"]=true, ["True"]=true,
  ["False"]=true, ["Nil"]=true, ["String"]=true, ["List"]=true,
  ["Result"]=true, ["Ok"]=true, ["Error"]=true, ["Option"]=true,
  ["Some"]=true, ["None"]=true, ["Image"]=true, ["Key"]=true,
  ["World"]=true, ["Style"]=true, ["Color"]=true, ["XPlace"]=true,
  ["YPlace"]=true, ["Point"]=true, ["Font"]=true,
}

local function escape(s)
  s = s:gsub("&", "&amp;")
  s = s:gsub("<", "&lt;")
  s = s:gsub(">", "&gt;")
  s = s:gsub('"', "&quot;")
  return s
end

local function tokenize(text)
  local result = {}
  local i = 1
  local len = #text

  while i <= len do
    -- String
    if text:sub(i, i) == '"' then
      local j = i + 1
      while j <= len and text:sub(j, j) ~= '"' do
        if text:sub(j, j) == '\\' then j = j + 1 end
        j = j + 1
      end
      table.insert(result, '<span class="st">' .. escape(text:sub(i, j)) .. '</span>')
      i = j + 1
    -- Word (keyword, type, function, or identifier)
    elseif text:sub(i, i):match("[%a_]") then
      local j = i
      while j <= len and text:sub(j, j):match("[%w_]") do j = j + 1 end
      local word = text:sub(i, j - 1)
      if keywords[word] then
        table.insert(result, '<span class="kw">' .. escape(word) .. '</span>')
      elseif types[word] or word:sub(1,1):match("[A-Z]") then
        table.insert(result, '<span class="dt">' .. escape(word) .. '</span>')
      elseif j <= len and text:sub(j, j) == '(' then
        table.insert(result, '<span class="fu">' .. escape(word) .. '</span>')
      else
        table.insert(result, escape(word))
      end
      i = j
    -- Number
    elseif text:sub(i, i):match("%d") then
      local j = i
      while j <= len and text:sub(j, j):match("[%d_.]") do j = j + 1 end
      table.insert(result, '<span class="dv">' .. escape(text:sub(i, j - 1)) .. '</span>')
      i = j
    -- Operators and punctuation
    elseif text:sub(i, i):match("[%(%)%[%]{},.:<>+%-%*/%%=|&#!]") then
      table.insert(result, '<span class="op">' .. escape(text:sub(i, i)) .. '</span>')
      i = i + 1
    else
      table.insert(result, escape(text:sub(i, i)))
      i = i + 1
    end
  end

  return table.concat(result)
end

function Code(el)
  if el.classes[1] == "gleam" then
    local html = tokenize(el.text)
    return pandoc.RawInline("html", '<code class="sourceCode gleam">' .. html .. '</code>')
  end
  return el
end
