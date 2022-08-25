"use strict";

function showUser(div,user) {
    const a = add(div,"a");
    a.innerText=user;
    a.href = "get_user?uid="+encodeURIComponent(user);
    div.append(" ");
    const editA = add(div,"a")
    editA.innerText="Edit";
    editA.href="EditUser.html?uid="+encodeURIComponent(user);
    div.append(" ");
    const qA = add(div,"a")
    qA.innerText="Questions authored";
    qA.href="get_questions_created_by_user?uid="+encodeURIComponent(user);
}
function updateUserList() {
    function success(data) {
        console.log(data);
        const div = document.getElementById("UserList");
        removeAllChildElements(div);
        if (data.Ok) for (const line of data.Ok) showUser(add(div,"div"),line);
        else if (data.Err) div.innerText="Error : "+data.Err;
    }
    getWebJSON("get_user_list",success,failure);
}


function addUser() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            check_signature(result.Ok);
            status("Added user successfully. Bulletin Board hash "+result.Ok.message+" signature "+result.Ok.signature);
        } else {
            status("Tried to add user. Got Error message "+result.Err);
        }
        updateUserList();
    }
    let message = {
        uid : document.getElementById("UID").value,
        display_name : document.getElementById("DisplayName").value,
        public_key : document.getElementById("PublicKey").value,
    }
    const state = document.getElementById("State").value;
    if (state!=="empty") message.state = state;
    const electorates = document.getElementById("Electorates").value;
    function describe_electorate(s) {
        let ss = s.split(",");
        if (ss.length===1) return { chamber : s };
        if (ss.length===2) return { chamber : ss[0], region : ss[1] };
        status("Illegal electorate "+s);
        return null;
    }
    if (electorates.length>0) message.electorates=electorates.split(';').map(describe_electorate);
    getWebJSON("new_registration",success,failure,JSON.stringify(message),"application/json")
}

window.onload = function () {
    document.getElementById("Add").onclick = addUser;
    updateUserList();
}