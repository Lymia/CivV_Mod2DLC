-- Copyright (c) 2015-2016 Lymia Alusyia <lymia@lymiahugs.com>
--
-- Permission is hereby granted, free of charge, to any person obtaining a copy
-- of this software and associated documentation files (the "Software"), to deal
-- in the Software without restriction, including without limitation the rights
-- to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
-- copies of the Software, and to permit persons to whom the Software is
-- furnished to do so, subject to the following conditions:
--
-- The above copyright notice and this permission notice shall be included in
-- all copies or substantial portions of the Software.
--
-- THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
-- IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
-- FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
-- AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
-- LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
-- OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
-- THE SOFTWARE.

local marker = "mppatch_command:yLsMoqQAirGJ4RBQv8URAwcu6RXdqN6v:"

local chatProtocolCommands = {}
function _mpPatch.registerChatCommand(id, fn)
    chatProtocolCommands[id] = fn
    return function(data)
        _mpPatch.sendChatCommand(id, data)
    end
end
function _mpPatch.sendChatCommand(id, data)
    Network.SendChat(marker..id..":"..(data or ""))
end

function _mpPatch.interceptChatFunction(fn, noCheckHide)
    local function chatFn(...)
        local fromPlayer, _, text = ...
        if (noCheckHide or not ContextPtr:IsHidden()) and m_PlayerNames[fromPlayer] then
            local textHead, textTail = text:sub(1, marker:len()), text:sub(marker:len() + 1)
            if textHead == marker then
                local split = textTail:find(":")
                local command, data = textTail:sub(1, split - 1), textTail:sub(split + 1)
                local fn = chatProtocolCommands[command]
                if not fn then
                    return
                else
                    return fn(data, ...)
                end
            end
        end
        return fn(...)
    end
    Events.GameMessageChat.Remove(fn)
    Events.GameMessageChat.Add(chatFn)
    return chatFn
end