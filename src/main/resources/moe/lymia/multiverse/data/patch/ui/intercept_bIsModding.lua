if _mpPatch and _mpPatch.isModding() then
    function ContextPtr.LookUpControl()
        return {
            GetID = function() return "ModMultiplayerSelectScreen" end
        }
    end
end
