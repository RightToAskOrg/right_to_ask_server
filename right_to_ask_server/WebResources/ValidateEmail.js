"use strict";

function getWhy() {
    switch (document.getElementById("Purpose").value) {
        case "MP" : return {"AsMP":true};
        case "MPStaffer" : return  {"AsMP":false};
        case "Org" : return "AsOrg";
        case "revokeMP" : return {"RevokeMP" : [document.getElementById("Revokee").value,true]};
        case "revokeMPStaffer" : return {"RevokeMP" : [document.getElementById("Revokee").value,false]};
        case "revokeOrg" : return {"RevokeOrg" : document.getElementById("Revokee").value};
    }
    failure("Need to say why");
    return null;
}

let email_id = null;


function RequestEmail() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            check_signature(result.Ok);
            email_id = result.Ok.message;
            status("Requested email successfully and verified signature. Email id "+email_id);
        } else {
            status("Tried request email. Got Error message "+result.Err);
        }
    }
    const command = {
        name : document.getElementById("BadgeName").value,
        why : getWhy()
    }
    let signed_message = sign_message(command);
    signed_message.email =  document.getElementById("Email").value;
    getWebJSON("request_email_validation",success,failure,JSON.stringify(signed_message),"application/json")
}

function VerifyCode() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            check_signature(result.Ok);
            status("Code verified successfully and verified signature. Bulletin board "+result.Ok.message);
        } else if (result.Err) {
            status("Tried request email. Got Error message "+result.Err);
        } else {
            status("Code seems to have verified successfully. No Bulletin board entry.")
        }
    }
    let command = {
        hash : email_id,
        code : parseInt(document.getElementById("Code").value,10)
    }
    let signed_message = sign_message(command);
    getWebJSON("email_proof",success,failure,JSON.stringify(signed_message),"application/json")
}

window.onload = function () {
    document.getElementById("RequestEmail").onclick = RequestEmail;
    document.getElementById("VerifyCode").onclick = VerifyCode;
    getWebJSON("MPs.json",function (mpList) {
        makePoliticianList("ChooseMP",mpList,function (mp) {
            document.getElementById("Email").value=mp.email;
            document.getElementById("BadgeName").value=mp.first_name+" "+mp.surname+" "+mp.email.slice(mp.email.indexOf("@"));
        });
    },failure);
}