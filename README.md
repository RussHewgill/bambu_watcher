
# Bambu Watcher

Bambu Watcher is a dashboard for monitoring the status of multiple bambulab printers.

## Features

- Drag and drop printers to arrange your dashboard
- Notifications on print error, completion, and pause
- Webcam stream for P1 and A1 series
- Bamub cloud login (Password not saved)
- Print preview thumbnails (only when logged in)

## Download

Get the latest build from the [releases page](https://github.com/RussHewgill/bambu_watcher/releases)

## Instructions

1. Create a file in the same directory as the program, named `config.yaml`
2. Paste the following template into it:
```yaml
printers:
- name: printer1
  host: XXX.XXX.XXX.XXX
  access_code: 12341234
  serial: XXXXXXXXXXXXXXX
- name: printer2
  host: XXX.XXX.XXX.XXX
  access_code: 56785678
  serial: XXXXXXXXXXXXXXX
```
3. For each P1S, go to the 3rd menu, then select "WLAN"
  - Copy the `IP` and `Access Code` to the `host` and `access_code` fields
  - Go to Bambu Studio/Orca Slicer, and copy the serial from the `device` tab in the `update` menu

## Known issues

- X1C has problems connecting

## Credits

Icons from [Icons8](https://icons8.com)

## If this is helpful to you, consider buying me a coffee:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/I3I1W8O4I)


