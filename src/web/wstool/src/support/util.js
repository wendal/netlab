import _ from 'lodash'

export default {
  abc() {
    console.log(_.indexOf(['A', 'B', 'C'], 'C'))
  },
  connect({
    host, 
    onopen, onmessage, onclose, onerror
  }={}) {
    let schm = ({"http:":"ws","https:":"wss"})[window.location.protocol]
    var wsUrl = `${schm}:${host}`
    console.log(wsUrl)
    // Create websocket
    let $WS = new WebSocket(wsUrl) 
    $WS.onopen = onopen
    $WS.onmessage = onmessage
    $WS.onclose = onclose
    $WS.onerror = onerror

    // Return the instance
    return $WS
  },
  decodeUtf8(bytes) {
    var encoded = "";
    for (var i = 0; i < bytes.length; i+=2) {
        encoded += '%' + bytes[i] + bytes[i+1]
    }
    return decodeURIComponent(encoded);
  },
  //---------------------------------------
  formatDate(date, fmt="yyyy-MM-dd HH:mm:ss.SSS") {
    if(!_.isDate(date)) {
      date = new Date(date)
    }
    // Guard it
    if(!date)
      return null
    
    // TODO here add another param
    // to format the datetime to "in 5min" like string
    // Maybe the param should named as "shorthand"
    /*
    E   :Mon
    EE  :Mon
    EEE :Mon
    EEEE:Monday
    M   :9
    MM  :09
    MMM :Sep
    MMMM:September
    */
    // Format by pattern
    let yyyy = date.getFullYear()
    let M = date.getMonth() + 1
    let d = date.getDate()
    let H = date.getHours()
    let m = date.getMinutes()
    let s = date.getSeconds()
    let S = date.getMilliseconds()

    let _c = {
      yyyy, M, d, H, m, s, S,
      yyy : yyyy,
      yy  : (""+yyyy).substring(2,4),
      MM  : _.padStart(M, 2, '0'),
      dd  : _.padStart(d, 2, '0'),
      HH  : _.padStart(H, 2, '0'),
      mm  : _.padStart(m, 2, '0'),
      ss  : _.padStart(s, 2, '0'),
      SS  : _.padStart(S, 3, '0'),
      SSS : _.padStart(S, 3, '0')
    }
    let regex = /(y{2,4}|M{1,4}|dd?|HH?|mm?|ss?|S{1,3}|E{1,4}|'([^']+)')/g;
    let ma;
    let list = []
    let last = 0
    ma=regex.exec(fmt)
    while(ma) {
      if(last < ma.index) {
        list.push(fmt.substring(last, ma.index))
      }
      let it = ma[2] || _c[ma[1]] ||  ma[1]
      list.push(it)
      last = regex.lastIndex
      ma=regex.exec(fmt)
    }
    if(last < fmt.length) {
      list.push(fmt.substring(last))
    }
    return list.join("")
  }

}