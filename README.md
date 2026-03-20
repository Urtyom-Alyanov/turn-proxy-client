# TURN Proxy

![GitHub License](https://img.shields.io/github/license/Urtyom-Alyanov/turn-proxy-client?style=for-the-badge&logo=gplv3&logoColor=FFFFFF)
![GitHub repo size](https://img.shields.io/github/repo-size/Urtyom-Alyanov/turn-proxy-client?style=for-the-badge&logo=github&logoColor=FFFFFF)

[//]: # "![Docker image size](https://img.shields.io/docker/image-size/ghcr/Urtyom-Alyanov/turn-proxy-client?style=for-the-badge&logo=docker&logoColor=FFFFFF)"

![GitHub top language](https://img.shields.io/github/languages/top/Urtyom-Alyanov/turn-proxy-client?style=for-the-badge&logo=rust&color=FF8000&logoColor=FFFFFF)
![GitHub branch status](https://img.shields.io/github/checks-status/Urtyom-Alyanov/turn-proxy-client/main?style=for-the-badge&logo=githubactions&logoColor=FFFFFF)
![Last Commit](https://img.shields.io/github/last-commit/Urtyom-Alyanov/turn-proxy-client?style=for-the-badge&logo=git&logoColor=FFFFFF)

## Отказ от ответственности (дисклеймер)

Данный проект является исследовательским, автор не несёт ответственности за использование его трудов для обхода
блокировок запрещённых сервисов. Также автор не ручается за нарушение пользовательского соглашения провайдеров сервисов,
предоставляющих услуги видеозвонков.
Поэтому советую вам использовать, пока что, [клиент от cacggghp](https://github.com/cacggghp/vk-turn-proxy)

## Что этот проект делает

Этот проект принимает и отсылает DTLS пакеты сквозь TURN сервера, используя их как прокси сервера меж условным клиентом и сервером, данный проект является именно первым.

Проще говоря, этот проект изучает как работают видеозвонки в компьютерной сети "Интернет", позволяя передавать через них
не только аудио и видео информацию, но вообще любую, при этом исключая MITM-атаку и анализ с помощью шифрования DTLS

```mermaid
graph LR
    subgraph Устройство пользователя
        A[UDP Клиент, например клиент WireGuard] -- Чистый UDP  --> B[DTLS обфускатор/Клиентский прокси/Это приложение]
    end
    B -- DTLS Tunnel --> C[TURN Сервер, например ВК звонки или Яндекс.Телемост]
    C -- DTLS Tunnel --> D[DTLS деобфускатор/Серверный прокси]
    subgraph Прокси-сервер
        D -- Чистый UDP --> E[UDP сервис, например WireGuard]
    end
```

## Развёртка

На данный момент доступны [`flake.nix`](./flake.nix) для пакетного менеджера Nix вместе с модулем для NixOS, а также
[`PKGBUILD`](./PKGBUILD) для Arch Linux. В Releases также доступны пакеты для Debian-based и RPM-based дистрибутивов. В скором времени может добавиться поддержка ОС Windows и Android.

<!--### Быстрая установка (Debian/Ubuntu/Fedora/и производные)
```bash
curl -sSL https://raw.githubusercontent.com/Urtyom-Alyanov/turn-proxy-client/master/install.sh | bash
```-->

#### Минимальные требования

- Ubuntu 21.10 (или 22.04 LTS)
- Debian 12 (Bookworm)
- Fedora 35
- RHEL / CentOS 9

Проще говоря, нужна минимальная версия glibc: `2.27`

README потом закончу
