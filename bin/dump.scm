(define (chars2atoms str)
  (map (lambda (c)
         (char->integer c))
       (string->list str)))

(define (improper-list->list v)
  (cond ((pair? v)
         (cons (car v)
               (improper-list->list (cdr v))))
        ((null? v)
         v)
        (else
         (list v))))

(define (dump v)
  (cond ((boolean? v) (if v 'true 'false))
        ((char? v) `(integer->char ,(char->integer v)))
        ((keyword? v) `(keyword2 ,@(chars2atoms (keyword->string v))))
        ((string? v) `(string ,@(chars2atoms v)))
        ((symbol? v) `(symbol ,@(chars2atoms (symbol->string v))))
        ((number? v) `(number ,v))
        ((list? v) `(list ,@(map dump v)))
        ((pair? v) `(improper-list ,@(map dump (improper-list->list v))))
        (else (error "dump: missing mapping for:" v))))

(for-each pretty-print
          (map dump
               (call-with-input-file (list path: (getenv "input_file")
                                           char-encoding: 'UTF-8)
                 read-all)))
(newline)
