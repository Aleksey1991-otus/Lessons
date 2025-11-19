# Инструкция по публикации репозитория

Для публикации проекта на GitHub выполните следующие шаги:

## 1. Создайте репозиторий на GitHub
- Перейдите на сайт https://github.com
- Нажмите кнопку "New repository"
- Укажите имя репозитория (например, "news-telegram-bot")
- Выберите "Public" или "Private" в зависимости от ваших предпочтений
- Не ставьте галочку "Initialize this repository with a README"
- Нажмите "Create repository"

## 2. Обновите URL удаленного репозитория
Замените URL ниже на URL вашего нового репозитория:

```bash
git remote set-url origin https://github.com/ваш-username/ваш-репозиторий.git
```

## 3. Отправьте изменения в удаленный репозиторий
```bash
git push -u origin qwen-code-60479ced-ccd8-43ec-941f-3c50209b3aee
```

## 4. Если хотите переименовать ветку на main
```bash
git branch -M main
git push -u origin main
```

## 5. Подключите ваш GitHub аккаунт
Для аутентификации может потребоваться использовать токен доступа:
- Перейдите в Settings -> Developer settings -> Personal access tokens -> Tokens (classic)
- Создайте новый токен с правами repo
- Используйте токен вместо пароля при аутентификации

## Структура проекта
Проект содержит:
- `src/main.rs` - основной код бота
- `Cargo.toml` - зависимости проекта
- `.env.example` - пример файла с переменными окружения
- `README.md` - документация
- `INSTALLATION_GUIDE.md` - инструкция по установке
- `ARCHITECTURE.md` - архитектурные решения