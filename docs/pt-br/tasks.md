# Task: efeitos web descritivos e executor de demonstração

## Escopo

A biblioteca descreve GET, POST e um despertar após um intervalo (`wake_up`).
O programa permanece funcional: init/update/view e as continuações calculam
valores e descrevem pedidos. O ambiente interpreta esses pedidos.

**Somente o executor fake está implementado.** Não há requisições reais,
temporizador real, agendador assíncrono ou renderizador desktop. O foco da UI é
web. As funções `task.write_file` e `task.println` foram removidas, junto com
seus construtores e respostas no fake. Isso não remove APIs de console da
linguagem nem a apresentação de resultados pelo ambiente.

O fake não acessa rede, disco ou relógio. Ele recebe respostas predefinidas e
produz resultado e histórico. A renderização HTML e a leitura de eventos da UI
continuam sendo responsabilidades reais do host.

## Decisões baseadas em Elm, JavaScript e Rust

### Elm

[Http.elm 2.0.0](https://github.com/elm/http/blob/2.0.0/src/Http.elm) usa o mesmo
`Http.Error` para GET e POST: BadUrl, Timeout, NetworkError, BadStatus e BadBody.
GET e POST delegam a uma requisição comum; POST acrescenta Body. `stringBody`
carrega conteúdo e MIME type, enquanto `expectString` espera texto e considera
sucesso os códigos 2xx. BadBody aparece ao interpretar um corpo incompatível com
a expectativa, como uma falha de decodificação JSON.

Nesta biblioteca, GET e POST também compartilham um enum de erros e POST tem
um Body explícito. Como a resposta é texto, não oferecemos um decodificador HTTP
nem a variante BadBody: decodificar JSON é uma etapa pura do programa.

[Process.sleep](https://github.com/elm/core/blob/1.0.5/src/Process.elm) descreve
uma espera em milissegundos antes de continuar. Nosso wake_up representa uma
espera única; o fake apenas confirma o pedido. Uma repetição precisa ser
explicitamente solicitada pelo programa.

### JavaScript no navegador

[Fetch](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API/Using_Fetch)
usa uma Promise e opções como method, body e headers. POST não tem uma classe
separada de erros. Uma resposta 404/500 não rejeita automaticamente a Promise:
o código precisa examinar `response.ok` ou `response.status`.

Nossa API escolhe expor esses status como `BadStatus`, seguindo a conveniência
do Elm. Um futuro executor web precisará fazer essa conversão explicitamente.
Erros de rede e restrições como CORS nem sempre podem ser distinguidos pelo
código do navegador; não prometemos variantes detalhadas de DNS, TLS ou CORS.

### Rust

Rust não define um cliente HTTP único. A comparação é com
[reqwest::Error](https://docs.rs/reqwest/latest/reqwest/struct.Error.html), que
oferece classificações como timeout, conexão, status e decodificação para
requisições, independentemente de GET ou POST.
[Response::error_for_status](https://docs.rs/reqwest/latest/reqwest/struct.Response.html#method.error_for_status)
transforma status 4xx/5xx em erros. Isso não é idêntico à regra escolhida aqui,
que só aceita 2xx no resultado final, como expectString do Elm.

Não adicionamos reqwest ao projeto: o escopo solicitado é web e fake. A pesquisa
orienta o contrato público, não representa a implementação de um cliente real.

## API pública

Arquivo: [task.gleam](../../lib/sgleam/task.gleam).

```gleam
pub type HttpError {
  BadUrl(url: String)
  Timeout
  NetworkError
  BadStatus(status: Int)
}

pub type Body {
  EmptyBody
  StringBody(content_type: String, content: String)
}

pub type Task(a) {
  None
  Done(value: a)
  Get(url: String, next: fn(Result(String, HttpError)) -> Task(a))
  Post(url: String, body: Body, next: fn(Result(String, HttpError)) -> Task(a))
  WakeUp(delay_ms: Int, next: fn(Nil) -> Task(a))
}
```

Assinaturas das funções:

```gleam
none() -> Task(a)
done(value: a) -> Task(a)
then(first: Task(a), next: fn(a) -> Task(b)) -> Task(b)
get(url: String, next: fn(Result(String, HttpError)) -> Task(a)) -> Task(a)
post(url: String, body: Body, next: fn(Result(String, HttpError)) -> Task(a)) -> Task(a)
wake_up(delay_ms: Int, next: fn(Nil) -> Task(a)) -> Task(a)
empty_body() -> Body
string_body(content_type: String, content: String) -> Body
```

As funções get/post/wake_up constroem descrições. Elas não chamam then nem
realizam a operação. O use fornece a continuação como último argumento:

```gleam
use response <- task.post(
  "https://example.com/notes",
  task.string_body("text/plain", "Minha anotação"),
)
task.done(GotResponse(response))
```

Esse trecho pressupõe um construtor de mensagem GotResponse. Para JSON, passe
`application/json` e uma String já serializada, por exemplo usando gleam/json.
`string_body` não codifica nem valida JSON. EmptyBody é diferente de StringBody
com conteúdo vazio, pois este último ainda especifica um tipo de conteúdo.

GET/POST produzem `Result(String, HttpError)`: um texto bem-sucedido ou erro
tipado. Não há resposta completa com headers/status de sucesso nesta versão.
O programa pode tratar cada erro com case, sem comparar textos de erro.

```gleam
case response {
  Ok(body) -> task.done(Loaded(body))
  Error(task.BadUrl(url)) -> task.done(Failed("URL inválida: " <> url))
  Error(task.Timeout) -> task.done(Failed("Tempo esgotado"))
  Error(task.NetworkError) -> task.done(Failed("Falha de rede"))
  Error(task.BadStatus(code)) -> task.done(Failed("HTTP " <> int.to_string(code)))
}
```

O trecho pressupõe as mensagens Loaded/Failed e os imports task e gleam/int.

## Wake-up

```gleam
fn next_tick() -> Task(Msg) {
  use _ <- task.wake_up(1000)
  task.done(Tick)
}
```

O trecho pressupõe Msg com a variante Tick. Significa pedir uma única
continuação após 1000 milissegundos. A resposta é Nil: o evento é o próprio
fim da espera. Intervalos negativos são normalizados para zero por wake_up;
zero significa sem atraso solicitado. Construir a tarefa não espera nada.

No fake, só `WakeUpResponse(1000)` confirma esse pedido. Uma resposta de 999
não corresponde. A confirmação é imediata; o fake não mede nem avança um
relógio virtual e não usa sleep. O histórico registra o intervalo solicitado.
Uma repetição infinita não deve ser usada com esse executor imediato.
[app_wake_up](../../examples/app_wake_up.gleam) usa uma contagem limitada a três
pedidos e para com stop_when.

## Composição

`then` conecta uma Task pronta ao próximo processamento. Em Done, chama a
próxima função; em pedidos, preserva-os e compõe a continuação; em None, termina
sem chamar a próxima função. O resultado final pode mudar de tipo.

```gleam
use value <- task.then(task.done(42))
task.done(int.to_string(value))
```

As operações básicas aceitam a continuação diretamente, portanto continuam
permitindo `use response <- task.get(url)` sem then explícito. None termina sem
valor/mensagem. Done produz um valor; no App, esse valor é uma mensagem.

## Executor fake e relatórios

Arquivo: [fake.gleam](../../lib/sgleam/task/fake.gleam).

```gleam
fake.GetResponse(url, Ok("texto"))
fake.GetResponse(url, Error(task.Timeout))
fake.PostResponse(url, task.string_body("text/plain", "nota"), Ok("recebido"))
fake.WakeUpResponse(1000)
```

O fake usa a primeira resposta correspondente, reutilizável. Não consome as
fixtures como fila. GET compara a URL. POST compara URL e Body completos,
incluindo MIME type e conteúdo. WakeUp compara o intervalo normalizado.
Uma resposta de GET não responde a um POST para a mesma URL.

`fake.http_response(status, body)` é uma função pura para construir fixtures:
200 a 299 viram Ok(body); outros códigos viram Error(BadStatus(status)). Ela
não faz HTTP, não segue redirecionamentos nem valida um servidor. Para um 204,
use corpo vazio. Pode ser usada em GetResponse e PostResponse.

| Função | Resultado |
| --- | --- |
| `run(task, responses)` | `Report(a)`, exige valor final |
| `run_messages(task, responses)` | `Report(List(a))`, aceita zero ou uma mensagem |
| `value(task, responses)` | Extrai o valor ou gera panic em falha |
| `messages(task, responses)` | Extrai mensagens ou gera panic em falha |
| `demo_responses()` | Fixtures da sequência GET -> POST -> wake_up |
| `demo_value(task)` / `demo_messages(task)` | Adaptadores com fixtures padrão |

Report contém `result: Result(a, Failure)` e `requests: List(Request)`.
O histórico segue a ordem dos pedidos, incluindo um eventual pedido rejeitado.
`run_messages` representa None com Ok([]), e Done(msg) com Ok([msg]). `run`
representa None como Error(NoValue), pois seu contrato exige um valor.

**Erros HTTP e falhas do fake são coisas diferentes.** Uma fixture com
Error(NetworkError) é uma resposta conhecida: a continuação recebe esse erro.
Um pedido desconhecido resulta em Error(UnexpectedRequest(request)) no relatório
e não chama a continuação. O fake nunca inventa uma resposta nem usa rede real.
Uma URL malformada só resulta em BadUrl se a fixture a configurar assim; esta
simulação não é um validador de URLs. Timeout também é um resultado configurado,
não um prazo contado por um temporizador.

## Integração MVU

| Função | SimpleApp | App |
| --- | --- | --- |
| init | `fn() -> model` | `fn() -> #(model, Task(msg))` |
| view | `fn(model) -> Element(msg)` | `fn(model) -> Element(msg)` |
| update | `fn(model, msg) -> model` | `fn(model, msg) -> #(model, Task(msg))` |

O `SimpleApp` é um wrapper que guarda um `App`. Seu construtor adapta `init`
e `update` para retornar `#(modelo, task.none())`, sem executar `init` antes de
`run`. `stop_when` e `run` delegam ao App. O tipo opaco e o ciclo de execução
ficam em `sgleam/show/internal/application`, com um modo interno definido pelo
construtor: App renderiza após cada atualização; SimpleApp processa os eventos
disponíveis antes de renderizar novamente, usando os handlers da tela original.
A API pública não oferece uma opção de modo. Somente o construtor interno com
callbacks que retornam apenas modelos habilita lotes, garantindo tarefas vazias.
`stop_when` interrompe o lote após a atualização que satisfaz o predicado e o
ciclo renderiza o modelo final. O SimpleApp não depende de respostas do fake.

O App apresenta o modelo, interpreta sua tarefa e entrega as mensagens ao update
com o modelo atual. Mensagens pendentes são processadas em ordem. Depois o loop
volta aos eventos da interface. `run_with` recebe `fn(Task(msg)) -> List(msg)`.

```gleam
app.create(init, view, update)
|> app.run_with(fn(program) { fake.messages(program, responses) })
```

`app.run` usa demo_messages. Para outros pedidos, forneça fixtures explícitas.
Os exemplos HTTP/notas convertem falhas do fake em mensagens SimulationError;
essas mensagens passam por update, sem o executor modificar o modelo.

`task.none()` é usado quando não há mensagem a produzir. `task.done(msg)`
provoca outra atualização: retornar a mesma mensagem repetidamente causa um
ciclo infinito. Cada update apresenta sua view; o fake síncrono não garante
que o navegador pinte todos os estados intermediários.

`stop_when` apresenta o modelo final e retorna antes da tarefa desse estado,
das mensagens pendentes e de novos eventos. Não interrompe interpretação já
em andamento. A view final permanece, sem fechar aba ou processo.

## Execução e migração

`main() -> Task(Int)` continua sendo reconhecido pelo ambiente sgleam e
interpretado com demo_value. O inteiro não é convertido em exit status.
`smain()` pode devolver Report ou uma lista de verificações ao ambiente.
O fake é Gleam puro e funciona na execução nativa e WASM; a interface interativa
é web. Não existe integração desktop de controles nativos.

```sh
cargo run -- examples/task_web.gleam
cargo run -- examples/task_fake.gleam
cargo test -p sgleam --test tasks
cargo test -p engine --test task_app
```

Migração:

- Remova write_file/println de Task e suas fixtures: não há aliases obsoletos.
- Envio ao servidor usa post com Body, não caminho de arquivo.
- Troque erros String por HttpError e faça case sobre suas variantes.
- Use wake_up para descrever uma espera; configure WakeUpResponse no fake.
- task_download foi substituído por task_web, sem download para disco.
- Mantenha init/update com a tupla e use messages/run_messages no App.
- Sincronize a biblioteca no engine do SimpleCode e reconstrua seu WASM.

Os testes cobrem sucesso, todas as variantes de erro, limites de status, POST
com comparação exata, wake-up conhecido/desconhecido, normalização de intervalo,
composição, ausência de mensagens, repetibilidade, parada e fluxo MVU. Os testes
de worker do SimpleCode verificam as views e interações usando o WASM real.

## Pureza e limites

Task/fake não usam FFI nem importam I/O. Suas continuações devem ser puras.
Gleam não impede que um usuário chame uma função impura dentro da continuação:
a separação é arquitetural, não um sistema de efeitos verificado pelo compilador.
Compare relatórios, não Tasks estruturalmente, pois Tasks contêm funções.

Um executor real web futuro precisará agendar HTTP e temporizadores sem bloquear
interações, mapear falhas para HttpError e respeitar o encerramento da aplicação.
Esse executor não faz parte desta etapa, por decisão explícita de manter o fake.
