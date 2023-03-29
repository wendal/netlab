# NetLab

## 功能简介

一句话, 给你一个随机的外网端口, 用于测试TCP/UDP链接

测试地址: https://netlab.luatos.com

TODO:
1. 支持 mqtt
2. 支持 quic

计划完成时间? 猴年马月

## 基本原理

不知道 ^_^, 自己翻源码吧.

## 我想XXX

1. 发现bug? 欢迎pull request
2. 有开发需求? 欢迎 pull request
3. 不会部署?源码讲解? 自助吧

## 源码说明

一个maven工程, eclipse/idea均可按maven项目导入

MainLauncher是入口,启动即可

**注意**

自行部署要公网IP和域名, 且修改如下文件中的后端地址

文件路径: src\web\wstool\src\components

```js
default: "//netlab.luatos.com/ws/netlab",
```

修改成你自己的域名, 要**公网可访问的**!!

然后还得修改页面显示

```html
<em v-if="myClientPort > 0">112.125.89.8:{{ myClientPort }}</em>
```

把`112.125.89.8` 改成你的公网ip

最后重新编译前端代码, src/web 目录有编译脚本


## 环境要求

* 必须JDK8+
* eclipse或idea等IDE开发工具,可选

## 配置信息位置

数据库配置信息,jetty端口等配置信息,均位于src/main/resources/application.properties

## 命令下启动

仅供测试用,使用mvn命令即可

```
// for windows
set MAVEN_OPTS="-Dfile.encoding=UTF-8"
mvn compile nutzboot:run

// for *uix
export MAVEN_OPTS="-Dfile.encoding=UTF-8"
mvn compile nutzboot:run
```

## 项目打包

```
mvn clean package nutzboot:shade
```

请注意,当前需要package + nutzboot:shade, 单独执行package或者nutzboot:shade是不行的


### 跳过测试
```
mvn clean package nutzboot:shade -Dmaven.test.skip=true
```

## 相关资源

* 论坛: https://nutz.cn
* 官网: https://nutz.io
* 一键生成NB的项目: https://get.nutz.io
