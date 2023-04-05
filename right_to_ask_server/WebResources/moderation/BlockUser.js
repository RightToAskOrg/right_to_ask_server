"use strict";


function set_block_status(block) {
    const command = {
        uid : document.getElementById("UID").value,
        block : block,
    };
    function success(result) {
        console.log(result);
        if (result.hasOwnProperty("Ok")) {
            status("Setting block status for "+command.uid+" to "+command.block+" successful");
        } else {
            status("Tried to set block status for "+command.uid+" to "+command.block+". Got Error message "+result.Err);
        }
    }
    getWebJSON("block_user",success,failure,JSON.stringify(command),"application/json")
}

window.onload = function () {
    document.getElementById("BlockUser").onclick = () => set_block_status(true);
    document.getElementById("UnblockUser").onclick = () => set_block_status(false);
}
