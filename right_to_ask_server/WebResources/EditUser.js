"use strict";

function showUser(div,user) {
    const a = add(div,"a");
    a.innerText=user;
    a.href = "get_user?uid="+encodeURIComponent(user);
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

function editUser() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            check_signature(result.Ok);
            status("Edited user successfully. Bulletin Board hash "+result.Ok.message+" signature "+result.Ok.signature);
            updateUser();
        } else {
            status("Tried to edit user. Got Error message "+result.Err);
        }
    }

    let command = {};
    let display_name = document.getElementById("DisplayName").value;
    if (display_name!==user.display_name) command.display_name=display_name;
    let state = document.getElementById("State").value;
    if (state==="empty") state=undefined;
    if (state!==user.state) command.state=state||null;
    const electorates = document.getElementById("Electorates").value;
    function describe_electorate(s) {
        let ss = s.split(",");
        if (ss.length===1) return { chamber : s };
        if (ss.length===2) return { chamber : ss[0], region : ss[1] };
        status("Illegal electorate "+s);
        return null;
    }
    if (electorates!==getUserElectorates()) {
        if (electorates.length>0) command.electorates=electorates.split(';').map(describe_electorate);
        else command.electorates=[];
    }
    let signed_message = sign_message(command);
    getWebJSON("edit_user",success,failure,JSON.stringify(signed_message),"application/json")
}

let uid = null;
let user = null;

function getUserElectorates() {
    if (user.electorates===undefined) return "";
    else return user.electorates.map(e=>e.chamber+","+e.region).join(";");
}
function setUser(userInfo) {
    console.log(userInfo);
    if (userInfo.Ok) {
        user = userInfo.Ok;
        document.getElementById("UID").innerText=user.uid;
        document.getElementById("PublicKey").innerText=user.public_key;
        document.getElementById("DisplayName").value=user.display_name;
        document.getElementById("State").value=user.state;
        document.getElementById("Electorates").value=getUserElectorates();
    } else if (userInfo.Err) failure("Error : "+data.Err);
}

function updateUser() {
    getWebJSON(getURL("get_user",{uid:uid}),setUser,failure);
}
window.onload = function () {
    uid = new URLSearchParams(window.location.search).get("uid");
    document.getElementById("Edit").onclick = editUser;
    updateUser();
}