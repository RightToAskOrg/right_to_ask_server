"use strict";

// Make a div into a list of links for politicians, from the MPs.json list politicianList, with given callback.

function makePoliticianList(elementIO,politicianList,callback) {
    let lastChamber = null;
    const div = add(document.getElementById(elementIO),"details");
    add(div,"summary").innerText="Choose Politician";
    for (const p of politicianList.mps) {
        if (lastChamber!==p.electorate.chamber) {
            add(div,"h6",p.electorate.chamber);
            lastChamber=p.electorate.chamber;
        } else div.append(" ");
        const a = add(div,"a");
        a.innerText = p.first_name+" "+p.surname;
        a.href = "#";
        a.onclick = function() { callback(p); return false; }
    }
}