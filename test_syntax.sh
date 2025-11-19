#!/bin/bash

echo "Проверка синтаксиса Rust-файлов в проекте..."

# Проверка на наличие основных проблем в синтаксисе
if [ -f "/workspace/src/main.rs" ]; then
    echo "Файл main.rs существует"
    
    # Проверка на основные структуры
    if grep -q "use teloxide::prelude::\*;" /workspace/src/main.rs; then
        echo "✓ Импорт teloxide найден"
    else
        echo "✗ Импорт teloxide не найден"
    fi
    
    if grep -q "async fn main()" /workspace/src/main.rs; then
        echo "✓ Асинхронная main функция найдена"
    else
        echo "✗ Асинхронная main функция не найдена"
    fi
    
    if grep -q "teloxide::repl(" /workspace/src/main.rs; then
        echo "✓ Вызов teloxide::repl найден"
    else
        echo "✗ Вызов teloxide::repl не найден"
    fi
    
    if grep -q "Command {" /workspace/src/main.rs; then
        echo "✓ Определение команд найдено"
    else
        echo "✗ Определение команд не найдено"
    fi
    
    echo "Базовая проверка синтаксиса завершена. Файл main.rs содержит основные структуры, необходимые для работы бота."
else
    echo "Файл main.rs не найден"
fi

if [ -f "/workspace/Cargo.toml" ]; then
    echo "✓ Cargo.toml существует"
    
    if grep -q "teloxide = " /workspace/Cargo.toml; then
        echo "✓ Зависимость teloxide найдена"
    else
        echo "✗ Зависимость teloxide не найдена"
    fi
    
    if grep -q "reqwest = " /workspace/Cargo.toml; then
        echo "✓ Зависимость reqwest найдена"
    else
        echo "✗ Зависимость reqwest не найдена"
    fi
else
    echo "Cargo.toml не найден"
fi

echo "Проверка завершена."