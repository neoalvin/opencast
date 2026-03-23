package org.opencast.tv.dlna

object XmlTemplates {

    fun buildDeviceDescription(friendlyName: String, udn: String, baseUrl: String): String {
        return """<?xml version="1.0" encoding="UTF-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <device>
    <deviceType>urn:schemas-upnp-org:device:MediaRenderer:1</deviceType>
    <friendlyName>$friendlyName</friendlyName>
    <manufacturer>OpenCast</manufacturer>
    <modelName>OpenCast Renderer</modelName>
    <modelDescription>OpenCast DLNA Media Renderer</modelDescription>
    <UDN>uuid:$udn</UDN>
    <serviceList>
      <service>
        <serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:AVTransport</serviceId>
        <controlURL>/AVTransport/control</controlURL>
        <eventSubURL>/AVTransport/event</eventSubURL>
        <SCPDURL>/AVTransport/scpd.xml</SCPDURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:RenderingControl:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:RenderingControl</serviceId>
        <controlURL>/RenderingControl/control</controlURL>
        <eventSubURL>/RenderingControl/event</eventSubURL>
        <SCPDURL>/RenderingControl/scpd.xml</SCPDURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ConnectionManager</serviceId>
        <controlURL>/ConnectionManager/control</controlURL>
        <eventSubURL>/ConnectionManager/event</eventSubURL>
        <SCPDURL>/ConnectionManager/scpd.xml</SCPDURL>
      </service>
    </serviceList>
  </device>
  <URLBase>$baseUrl</URLBase>
</root>"""
    }

    fun buildLastChangeXml(transportState: String, volumePercent: Int, muted: Boolean): String {
        val muteVal = if (muted) "1" else "0"
        return """<Event xmlns="urn:schemas-upnp-org:metadata-1-0/AVT/">
  <InstanceID val="0">
    <TransportState val="$transportState"/>
    <CurrentTransportActions val="Play,Pause,Stop,Seek"/>
    <Volume channel="Master" val="$volumePercent"/>
    <Mute channel="Master" val="$muteVal"/>
  </InstanceID>
</Event>"""
    }

    fun buildSoapResponse(action: String, serviceUrn: String, body: String): String {
        return """<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:${action}Response xmlns:u="$serviceUrn">
      $body
    </u:${action}Response>
  </s:Body>
</s:Envelope>"""
    }
}
