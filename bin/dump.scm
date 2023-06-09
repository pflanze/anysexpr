(define (read-all-expr port)
  (let rec ()
    (let ((expr (##read-expr-from-port port)))
      (if (eof-object? expr)
          '()
          (cons expr (rec))))))

(define (cj-desourcify x)
  (let ((x (if (##source? x) (##source-code x) x)))
    (cond ((pair? x)
           (cons (cj-desourcify (car x))
                 (cj-desourcify (cdr x))))
          ((##vector? x)
           (vector-map-1 cj-desourcify x))
          ((box? x)
           (box (cj-desourcify (unbox x))))
          ;; XX more?
          (else
           x))))

(define (position-line v)
  (+ 1 (bitwise-and v 65535)))

(define (position-column v)
  (+ 1 (quotient v 65536)))


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
        ((uninterned-symbol? v) `(uninterned-symbol ,@(chars2atoms (symbol->string v))))
        ((symbol? v) `(symbol ,@(chars2atoms (symbol->string v))))
        ((number? v) `(number ,v))
        ((list? v) `(list ,@(map dump v)))
        ((pair? v) `(improper-list ,@(map dump (improper-list->list v))))
        (else
         (let ((sp (case v
                     ((#!eof) "eof")
                     ((#!void) "void")
                     ((#!optional) "optional")
                     ((#!rest) "rest")
                     ((#!key) "key")
                     (else #f))))
           (if sp
               `(special ,@(chars2atoms sp))
               (error "dump: missing mapping for:" v))))))

(for-each (lambda (v)
            (let* ((loc (##source-locat v))
                   (pos (##locat-position loc))
                   (line (position-line pos)))
              (pretty-print `(line ,line)))
            (pretty-print (dump (cj-desourcify v))))
          (call-with-input-file (list path: (getenv "input_file")
                                      char-encoding: 'UTF-8)
            read-all-expr))
(newline)
