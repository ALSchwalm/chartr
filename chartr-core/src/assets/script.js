function renderMicros(us) {
    if (us < 1000) {
        return `${us}us`
    } else if (us < 1000000) {
        return `${us / 1000}ms`
    } else {
        return `${us / 1000000}s`
    }
}

function updateIndicator(x, y) {
    let indicator = document.getElementById("indicator");
    indicator.setAttribute("x", x);
    indicator.setAttribute("y", __HEADING_HEIGHT__);

    let text = document.getElementById("indicator-text");
    text.setAttribute("x", x + 10);
    text.setAttribute("y", y - 10);
    text.innerHTML = renderMicros( (x - __LEFT_OFFSET__) * __US_PER_PIXEL__)
    lastMousePos = {
        x: x,
        y: y
    }
}

var lastMousePos = {x: 0, y: 0};
var lastScrollPos = {top: 0, left: 0};
document.addEventListener("mousemove", (e)=>{
    updateIndicator(e.pageX, e.pageY);
});

document.addEventListener("scroll", (e)=>{
    var xOffset = 0;
    var yOffset = 0
    if (document.documentElement.scrollTop != lastScrollPos.top) {
        yOffset = document.documentElement.scrollTop - lastScrollPos.top;
    }
    if (document.documentElement.scrollLeft != lastScrollPos.left) {
        xOffset = document.documentElement.scrollLeft - lastScrollPos.left;
    }

    updateIndicator(lastMousePos.x + xOffset, lastMousePos.y + yOffset);

    lastScrollPos = {
        top: document.documentElement.scrollTop,
        left: document.documentElement.scrollLeft
    }
});
