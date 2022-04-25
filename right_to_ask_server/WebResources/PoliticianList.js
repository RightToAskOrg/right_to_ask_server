"use strict";

// Make a div into a list of links for politicians, from the MPs.json list politicianList, with given callback.

function makePoliticianList(elementIO,politicianList,callback) {
    let lastChamber = null;
    const div = add(document.getElementById(elementIO),"details");
    add(div,"summary").innerText="Choose Politician";
    for (const p of politicianList.mps) {
        if (lastChamber!==p.electorate.chamber) {
            add(div,"h6").innerText=p.electorate.chamber;
            lastChamber=p.electorate.chamber;
        } else div.append(" ");
        const a = add(div,"a");
        a.innerText = p.first_name+" "+p.surname;
        a.href = "#";
        a.onclick = function() { callback(p); return false; }
    }
}

// Make a div into a list of links for committees, from the committees.json list committeeList, with given callback.

function makeCommitteeList(elementIO,committeeList,callback) {
    let lastChamber = null;
    const div = add(document.getElementById(elementIO),"details");
    add(div,"summary").innerText="Choose Committee";
    for (const p of committeeList) {
        if (lastChamber!==p.jurisdiction) {
            add(div,"h6").innerText=p.jurisdiction;
            lastChamber=p.jurisdiction;
        } else div.append(" ");
        const a = add(div,"a");
        a.innerText = p.name;
        a.href = "#";
        a.onclick = function() { callback(p); return false; }
    }
}