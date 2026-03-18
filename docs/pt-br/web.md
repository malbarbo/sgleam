O playground web permite usar o sgleam diretamente no navegador, sem instalar nada.


# Layout

A interface é dividida em dois painéis:

- **Painel do editor** (esquerda ou topo): onde você escreve o código Gleam
- **Painel do REPL** (direita ou embaixo): onde a saída é exibida

O layout inicial é escolhido automaticamente com base nas dimensões da tela: horizontal para telas largas e vertical para telas altas.


# Barra de ferramentas

- **Run** (▶): Formata e executa as definições
- **Stop** (■): Interrompe a execução
- **Tema** (☀): Alterna entre os temas claro e escuro
- **Layout**: Alterna entre layout horizontal e vertical


# Atalhos de teclado

| Atalho | Descrição |
|--------|-----------|
| `Ctrl+r` | Executa as definições |
| `Ctrl+f` | Formata o código |
| `Ctrl+j` | Foca no painel do editor |
| `Ctrl+k` | Foca no painel do REPL |
| `Ctrl+d` | Mostra/esconde o painel do editor |
| `Ctrl+i` | Mostra/esconde o painel do REPL |
| `Ctrl+l` | Alterna entre layout horizontal e vertical |
| `Ctrl+t` | Alterna entre tema claro e escuro |
| `Ctrl+?` | Mostra a janela de ajuda |
| `Esc` | Fecha a janela de ajuda |


# Como usar

1. Escreva suas definições no painel do editor
2. Pressione `Ctrl+r` ou clique em **Run**
3. Use o REPL para avaliar expressões usando as definições

O botão **Run** (ou `Ctrl+r`) formata o código, executa os testes (funções `_examples`{.gleam}) e carrega as definições no REPL. Depois disso, você pode chamar as funções definidas no editor diretamente no REPL.

O REPL funciona como o REPL da linha de comando: você pode digitar expressões, definições de variáveis, funções e tipos.


# Temas

O playground suporta dois temas baseados no editor Zed:

- **One Light** — tema claro (padrão)
- **One Dark** — tema escuro

A preferência é salva no navegador.
