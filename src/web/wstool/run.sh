#! /bin/bash
mode=$1
target=$2

if [ "$mode" == "i18n" ];then
    if [ "$target" == "en" ];then
      cp src/i18n/en-us.js src/i18n/current.js
    fi

    if [ "$target" == "cn" ];then
      cp src/i18n/zh-cn.js src/i18n/current.js
    fi
fi


if [ "$mode" == "build" ];then
    if [ "$target" == "en" ];then
      cp src/i18n/en-us.js src/i18n/current.js
      npm run build
      cp -r dist/* ../../main/resources/static_en/
    fi

    if [ "$target" == "cn" ];then
      cp src/i18n/zh-cn.js src/i18n/current.js
      cp -r dist/* ../../main/resources/static/
    fi
fi