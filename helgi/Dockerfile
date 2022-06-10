FROM prantlf/alpine-make-gcc:latest AS sigrun_builder
WORKDIR /app
COPY ./sigrun .
RUN apk add libx11 libxdmcp mesa-dev
RUN make

FROM golang:1.17-alpine AS helgi_builder
WORKDIR /app
COPY ./helgi .
RUN cd cmd && go build -o /helgi

FROM alpine:latest
COPY --from=helgi_builder /helgi /helgi
COPY --from=sigrun_builder /app/sigrun /sigrun
CMD [ "/helgi" ]
