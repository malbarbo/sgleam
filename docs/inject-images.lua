-- Pandoc Lua filter to inject SVG images from docs/images.svg
-- Usage in markdown: ![](img:rectangle)

local images = {}

-- Parse the images.svg file
local f = io.open("docs/images.svg", "r")
if f then
    local content = f:read("*a")
    f:close()
    local current_name = nil
    local current_lines = {}
    for line in content:gmatch("[^\n]+") do
        local name = line:match("^<!%-%-IMG:(.-)%-%->$")
        if name then
            if current_name then
                images[current_name] = table.concat(current_lines, "\n")
            end
            current_name = name
            current_lines = {}
        else
            table.insert(current_lines, line)
        end
    end
    if current_name then
        images[current_name] = table.concat(current_lines, "\n")
    end
end

function Image(el)
    local name = el.src:match("^img:(.+)$")
    if name and images[name] then
        return pandoc.RawInline("html", '<div class="image-example">' .. images[name] .. '</div>')
    end
    return el
end
