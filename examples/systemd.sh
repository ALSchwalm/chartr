#!/bin/bash

set -e

function duration_between() {
    echo "$(($2 - $1))"
}

CMD="cargo run --"
OUT=out.svg

initrd=$(systemctl show -p InitRDTimestampMonotonic --value)
userspace=$(systemctl show -p UserspaceTimestampMonotonic --value)
loader=$(systemctl show -p LoaderTimestampMonotonic --value)
firmware=$(systemctl show -p FirmwareTimestampMonotonic --value)

rm -f ${OUT}
${CMD} ${OUT} create --heading "$(uname -a)"

${CMD} ${OUT} add-actor -- "firmware"
${CMD} ${OUT} add-event --color "rgb(150,150,150)" -- \
               "firmware" -${firmware} $(duration_between ${loader} ${firmware})

${CMD} ${OUT} add-actor -- "loader"
${CMD} ${OUT} add-event --color "rgb(150,150,150)" -- \
               "loader" -${loader} ${loader}

${CMD} ${OUT} add-actor -- "kernel"
${CMD} ${OUT} add-event --color "rgb(150,150,150)" -- \
               "kernel" 0 ${initrd}

${CMD} $OUT add-actor -- "initrd"
${CMD} $OUT add-event --color "rgb(150,150,150)" -- \
               "initrd" ${initrd} $(duration_between ${initrd} ${userspace})

default=$(systemctl show -p ActiveEnterTimestampMonotonic --value -- default.target)

for service in $(systemctl list-units --all -o json | jq -r ".[].unit"); do
    activating=$(systemctl show -p InactiveExitTimestampMonotonic --value -- ${service})
    activated=$(systemctl show -p ActiveEnterTimestampMonotonic --value -- ${service})
    deactivating=$(systemctl show -p ActiveExitTimestampMonotonic --value -- ${service})
    deactivated=$(systemctl show -p InactiveEnterTimestampMonotonic --value -- ${service})

    # Skip anything that activated after the default target
    if [ ${activating} -gt ${default} ] || [ ${activating} -eq 0 ]; then
        continue
    fi

    ${CMD} ${OUT} add-actor -- ${service}

    activating_end=${activated}

    # If the unit never actually activated, then the activating time ends
    # when it deactivated
    if [ ${activated} -eq 0 ]; then
        activating_end=${deactivated}
    fi

    ${CMD} ${OUT} add-event --color "rgb(255,0,0)" -- \
           ${service} ${activating} \
           $(duration_between ${activating} ${activating_end}) \

    # If the unit never actually activated, don't add any other event
    if [ ${activated} -eq 0 ]; then
        continue
    fi

    # If the unit never stopped being active, then it is "endless"
    if [ ${deactivating} -eq 0 ] || [ ${activated} -gt ${deactivating} ]; then
        ${CMD} ${OUT} add-event --endless --color "rgb(200,150,150)" -- \
               ${service} ${activated}
    else
        if [ ${deactivating} -gt ${default} ]; then
            deactivating=${default}
        fi

        ${CMD} ${OUT} add-event --color "rgb(200,150,150)" -- \
               ${service} ${activated} $(duration_between ${activated} ${deactivating})
    fi
done
