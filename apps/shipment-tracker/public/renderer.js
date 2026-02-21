// Client-side renderer for delta mode — renders event cards in the browser
// without V8 re-rendering. ~600 bytes minified.
Magnetic.registerRenderer("events", function(ev) {
  var p = ev.params || {};
  var et = ev.event_type || ev.type || "unknown";
  var c = p.carrier || "";
  var tk = p.tracking_number || "";
  var ts = ev.timestamp_ms || ev.timestamp;
  var time = "";
  if (ts) {
    var d = new Date(ts);
    time = ("0"+d.getHours()).slice(-2)+":"+("0"+d.getMinutes()).slice(-2)+":"+("0"+d.getSeconds()).slice(-2);
  }

  // Event type badge class
  var etc = "bg-raised fg-text";
  if (et.indexOf("delivered") >= 0) etc = "bg-success fg-surface";
  else if (et.indexOf("transit") >= 0 || et.indexOf("pickup") >= 0) etc = "bg-info fg-surface";
  else if (et.indexOf("exception") >= 0 || et.indexOf("alert") >= 0) etc = "bg-danger fg-heading";

  // Carrier badge class
  var cc = c.toLowerCase();
  var ccl = cc === "ups" ? "bg-warning fg-surface" : cc === "fedex" ? "bg-info fg-surface" : cc === "usps" ? "bg-primary fg-heading" : "bg-raised fg-muted";

  // Build badges row
  var badges = [
    {tag:"span", attrs:{class:"text-xs bold px-sm py-xs round-sm " + etc}, text: et}
  ];
  if (c) {
    badges.push({tag:"span", attrs:{class:"text-xs bold px-sm py-xs round-sm uppercase " + ccl}, text: c});
  }
  badges.push({tag:"span", attrs:{class:"text-xs fg-muted"}, text: time});

  // Build info stack
  var infoChildren = [{tag:"div", attrs:{class:"row gap-sm items-center"}, children: badges}];
  if (tk) {
    infoChildren.push({tag:"span", attrs:{class:"text-sm font-mono fg-subtle"}, text: tk});
  }

  // Return DomNode descriptor (same structure as server-side EventCard)
  return {
    tag: "div",
    key: ev.event_id,
    attrs: {class: "row gap-md items-center p-md bg-raised border round-md"},
    children: [
      {tag:"div", attrs:{class:"stack gap-xs grow"}, children: infoChildren},
      {tag:"span", attrs:{class:"text-xs fg-muted font-mono truncate", style:"max-width:120px"}, text: (ev.event_id||"").slice(0,12)}
    ]
  };
});
