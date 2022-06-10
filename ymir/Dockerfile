FROM golang:1.17-alpine AS builder
WORKDIR /app
COPY . .
RUN cd cmd && go build -o /ymir

FROM alpine:latest
COPY --from=builder /ymir /ymir
CMD [ "/ymir" ]
